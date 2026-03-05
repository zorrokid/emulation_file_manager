use std::{
    collections::VecDeque,
    ffi::{CString, c_void},
    sync::{Arc, Mutex, OnceLock},
};

use crate::{
    ffi::{
        RetroGameGeometry, RetroPixelFormat, RETRO_ENVIRONMENT_GET_LOG_INTERFACE,
        RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY, RETRO_ENVIRONMENT_GET_VARIABLE,
        RETRO_ENVIRONMENT_SET_GEOMETRY, RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
    },
    frame_buffer::FrameBuffer,
    input::InputState,
};

/// All state that the extern "C" callbacks need to access.
///
/// Callbacks are free functions with no `self` — they cannot carry state as
/// method parameters. The standard solution is a process-wide static that the
/// callbacks look up at runtime. We use an Arc<Mutex<_>> for the frame buffer
/// and input state so the GTK draw callback and event handler can share them
/// without going through this global.
pub struct CoreCallbackState {
    pub frame_buffer: Arc<Mutex<FrameBuffer>>,
    pub input_state: Arc<Mutex<InputState>>,
    pub pixel_format: RetroPixelFormat,
    /// The system directory path as a null-terminated C string.
    /// Stored here (not as a local) so the *const c_char pointer we hand to
    /// the core in GET_SYSTEM_DIRECTORY stays valid for the lifetime of the core.
    /// If this were a local variable the pointer would dangle after the function returned.
    pub system_directory: CString,
    /// Stereo interleaved f32 samples pushed by the audio callbacks and drained
    /// by the cpal audio thread. Arc<Mutex<_>> because both sides run on
    /// different threads. We store f32 (not i16) so the cpal callback doesn't
    /// need to know the original format — conversion happens on push.
    ///
    /// We share only the buffer here (not the AudioOutput/cpal Stream) because
    /// cpal::Stream contains raw pointers and is not Sync, so it cannot live
    /// inside a static. LibretroCore owns the AudioOutput to keep the stream alive.
    pub audio_buffer: Arc<Mutex<VecDeque<f32>>>,
}

/// Process-wide singleton holding the active core's callback state.
///
/// OnceLock initialises the Mutex exactly once (on first access). The inner
/// Option is what we replace on each load/unload — Some while a core is
/// running, None after shutdown. This pattern lets us reuse the same static
/// across multiple load/unload cycles without re-initialising the OnceLock.
static CORE_STATE: OnceLock<Mutex<Option<CoreCallbackState>>> = OnceLock::new();

fn state_mutex() -> &'static Mutex<Option<CoreCallbackState>> {
    CORE_STATE.get_or_init(|| Mutex::new(None))
}

/// Install state before calling retro_init(). The core calls environment_cb
/// immediately during init, so the state must be in place beforehand.
pub fn install_state(state: CoreCallbackState) {
    *state_mutex().lock().expect("install_state lock") = Some(state);
}

/// Clear state after retro_deinit(). Drops the frame buffer and input state
/// Arcs held here; the GTK window may still hold its own Arcs briefly.
pub fn remove_state() {
    *state_mutex().lock().expect("remove_state lock") = None;
}

/// Run a closure with mutable access to the callback state.
/// Returns None if no core is currently loaded.
pub fn with_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut CoreCallbackState) -> R,
{
    state_mutex().lock().ok()?.as_mut().map(f)
}

// ---------------------------------------------------------------------------
// Callbacks — all must be `unsafe extern "C"` so the C core can call them.
//
// `extern "C"` = use the C calling convention.
// `unsafe`     = we promise Rust that we handle raw pointers correctly inside.
// ---------------------------------------------------------------------------

/// The main frontend↔core communication channel.
/// The core calls this during retro_init() and retro_load_game() to query
/// capabilities and hand us configuration. `cmd` selects the operation;
/// `data` is a type-erased pointer whose meaning depends on `cmd`.
pub unsafe extern "C" fn environment_cb(cmd: u32, data: *mut c_void) -> bool {
    match cmd {
        // Core asks: "where are BIOS / system files?"
        // data is *mut *const c_char — a pointer to a pointer.
        // We write our system directory pointer into the slot data points at,
        // so the core can read it back after we return.
        RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY => {
            with_state(|s| {
                // Cast the void pointer to the concrete type the API specifies.
                let ptr_to_ptr = data as *mut *const std::os::raw::c_char;
                // Write our CString's pointer into the slot.
                // SAFETY: data is a valid *mut *const c_char per the libretro spec.
                unsafe { *ptr_to_ptr = s.system_directory.as_ptr() };
            })
            .is_some()
        }

        // Core tells us which pixel format it will use for video output.
        // data is *const u32 containing a RetroPixelFormat discriminant.
        // We return true to accept the format, false to reject it.
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
            // SAFETY: data is a valid *const u32 per the libretro spec.
            let format_id = unsafe { *(data as *const u32) };
            let format = match format_id {
                0 => RetroPixelFormat::Rgb1555,
                1 => RetroPixelFormat::Xrgb8888,
                2 => RetroPixelFormat::Rgb565,
                _ => return false,
            };
            // Accept whichever format the core prefers. frame_buffer.rs handles
            // all three (XRGB8888, RGB565, RGB1555). Rejecting a format causes
            // the core to keep using it anyway while our pixel_format field
            // stays wrong, producing doubled / corrupt images.
            with_state(|s| s.pixel_format = format);
            true
        }

        // Core asks for a logging function pointer.
        // Defining a variadic extern "C" fn in Rust requires nightly
        // (#![feature(c_variadic)]), so we skip this for now.
        // The core will just not be able to log through our frontend.
        RETRO_ENVIRONMENT_GET_LOG_INTERFACE => false,

        // Core asks for the value of a named option (core-specific settings).
        // Returning false means "not supported" — the core uses its defaults.
        RETRO_ENVIRONMENT_GET_VARIABLE => false,

        // Core tells us its output geometry changed (e.g. right after load).
        // data is *const RetroGameGeometry. We resize the frame buffer to match.
        RETRO_ENVIRONMENT_SET_GEOMETRY => {
            // SAFETY: data is a valid *const RetroGameGeometry per the libretro spec.
            let geom = unsafe { &*(data as *const RetroGameGeometry) };
            with_state(|s| {
                let mut fb = s.frame_buffer.lock().expect("frame buffer lock");
                fb.width = geom.base_width;
                fb.height = geom.base_height;
                let new_len = (geom.base_width * geom.base_height * 4) as usize;
                fb.rgba_data.resize(new_len, 0);
            });
            true
        }

        // Any command we don't recognise: return false to signal "not supported".
        // Cores are required to handle this gracefully for optional features.
        _ => false,
    }
}

/// Called by the core once per frame with the rendered pixel data.
/// `pitch` is bytes per row (may be > width * bytes_per_pixel due to alignment padding).
pub unsafe extern "C" fn video_refresh_cb(
    data: *const c_void,
    width: u32,
    height: u32,
    pitch: usize,
) {
    with_state(|s| {
        let format = s.pixel_format;
        let mut fb = s.frame_buffer.lock().expect("frame buffer lock");
        fb.update(data, width, height, pitch, format);
    });
}

/// Push a slice of stereo interleaved i16 samples into the shared audio buffer.
///
/// i16 range is −32768..=32767; we normalise to −1.0..=1.0 for cpal.
/// We cap the buffer to avoid unbounded growth if cpal falls behind.
fn push_audio(samples: &[i16]) {
    with_state(|s| {
        let mut buf = s.audio_buffer.lock().expect("audio buffer lock");
        const MAX_BUFFERED: usize = 16384;
        for &s in samples {
            if buf.len() < MAX_BUFFERED {
                buf.push_back(s as f32 / 32768.0);
            }
        }
    });
}

/// Called by the core to output a single stereo sample pair.
/// Some cores use this; others use the batch variant exclusively.
pub unsafe extern "C" fn audio_sample_cb(left: i16, right: i16) {
    push_audio(&[left, right]);
}

/// Called by the core to output a batch of stereo interleaved samples.
/// `data` points to `frames * 2` i16 values: [L0, R0, L1, R1, …].
/// Must return the number of frames consumed (we always consume all of them).
pub unsafe extern "C" fn audio_sample_batch_cb(data: *const i16, frames: usize) -> usize {
    // SAFETY: the core guarantees `data` points to `frames * 2` valid i16 values.
    let samples = unsafe { std::slice::from_raw_parts(data, frames * 2) };
    push_audio(samples);
    frames
}

/// Called once per frame before input_state_cb so the frontend can snapshot
/// input devices. We push input from GTK events instead, so nothing to do here.
pub unsafe extern "C" fn input_poll_cb() {}

/// Called by the core to read the state of a single button/axis.
/// Returns 1 (pressed) or 0 (released) for digital buttons.
/// `port` = controller port (0 = player 1), `device` = device type,
/// `index` = sub-device index, `id` = button ID (JOYPAD_* constants).
pub unsafe extern "C" fn input_state_cb(
    port: u32,
    device: u32,
    _index: u32,
    id: u32,
) -> i16 {
    // RETRO_DEVICE_JOYPAD = 1. We only support player 1's joypad.
    const RETRO_DEVICE_JOYPAD: u32 = 1;
    if port != 0 || device != RETRO_DEVICE_JOYPAD {
        return 0;
    }
    with_state(|s| {
        s.input_state
            .lock()
            .expect("input state lock")
            .get_button(id) as i16
    })
    .unwrap_or(0)
}
