use std::{
    ffi::{CString, c_void},
    path::Path,
    sync::{Arc, Mutex},
};

use libloading::{Library, Symbol};

use crate::{
    audio::AudioOutput,
    callbacks::{self, CoreCallbackState},
    error::LibretroError,
    ffi::{
        RetroAudioSampleBatchFn, RetroAudioSampleFn, RetroEnvironmentFn, RetroGameInfo,
        RetroInputPollFn, RetroInputStateFn, RetroPixelFormat, RetroSystemAvInfo, RetroSystemInfo,
        RetroVideoRefreshFn,
    },
    frame_buffer::FrameBuffer,
    input::InputState,
};

pub struct LibretroCore {
    // The loaded .so library. Keeping this alive prevents dlclose() from being
    // called, which would invalidate every function pointer we resolved from it.
    // The underscore prefix tells Rust we intentionally hold it only for its
    // drop behaviour, not to call any methods on it directly.
    _library: Library,

    // Function pointers resolved from the library. Only the ones we call after
    // init are stored here — the rest are only needed during load() and are
    // used as locals there.
    retro_run: unsafe extern "C" fn(),
    retro_unload_game: unsafe extern "C" fn(),
    retro_deinit: unsafe extern "C" fn(),

    // Shared with the GTK draw callback and event handler respectively.
    // Arc<Mutex<_>> lets multiple owners each lock independently.
    pub frame_buffer: Arc<Mutex<FrameBuffer>>,
    pub input_state: Arc<Mutex<InputState>>,

    /// Keeps the cpal audio stream alive. Dropping this stops audio output.
    /// The stream's sample buffer is shared with the audio callbacks via
    /// CoreCallbackState.audio_buffer — they are two Arcs pointing to the
    /// same VecDeque. None if no audio device was available at load time.
    _audio_output: Option<AudioOutput>,

    /// Frames per second reported by the core after retro_load_game().
    /// Used to set the glib::timeout_add_local interval (~16.6ms for NTSC NES).
    pub fps: f64,
}

impl LibretroCore {
    /// Load a libretro core `.so`, initialise it and load a ROM.
    ///
    /// Must be called from the main thread — `retro_run()` must later be
    /// called from the same thread that called `retro_init()`.
    pub fn load(
        core_path: &Path,
        rom_path: &Path,
        system_dir: &Path,
        input_state: Arc<Mutex<InputState>>,
    ) -> Result<Self, LibretroError> {
        // dlopen() the shared library. All symbols resolved below borrow from
        // `lib`, so `lib` must not be moved or dropped until after we've copied
        // every function pointer out of its Symbols.
        //
        // SAFETY: loading a native library is inherently unsafe — the OS maps
        // arbitrary code into our process. We accept this as a fundamental
        // requirement of the libretro plugin model.
        let lib = unsafe { Library::new(core_path) }?;

        // Helper that resolves a symbol and immediately copies the function
        // pointer out, dropping the Symbol (and its borrow of `lib`) before
        // the next line. Function pointers are Copy so the deref is a copy,
        // not a move — no lifetime is attached to the resulting value.
        //
        // The symbol name must end with `\0` (null terminator) because
        // libloading passes it directly to dlsym() as a C string.
        macro_rules! load_fn {
            ($name:literal, $ty:ty) => {{
                let sym: Symbol<$ty> = unsafe { lib.get($name) }?;
                // `*sym` dereferences through Symbol's Deref impl to get a
                // copy of the fn pointer. After this block sym is dropped,
                // releasing the borrow on lib.
                *sym
            }};
        }

        // Resolve every symbol we need before touching the core.
        let retro_set_environment = load_fn!(
            b"retro_set_environment\0",
            unsafe extern "C" fn(RetroEnvironmentFn)
        );
        let retro_init = load_fn!(b"retro_init\0", unsafe extern "C" fn());
        let retro_deinit = load_fn!(b"retro_deinit\0", unsafe extern "C" fn());
        let retro_get_system_info = load_fn!(
            b"retro_get_system_info\0",
            unsafe extern "C" fn(*mut RetroSystemInfo)
        );
        let retro_set_video_refresh = load_fn!(
            b"retro_set_video_refresh\0",
            unsafe extern "C" fn(RetroVideoRefreshFn)
        );
        let retro_set_audio_sample = load_fn!(
            b"retro_set_audio_sample\0",
            unsafe extern "C" fn(RetroAudioSampleFn)
        );
        let retro_set_audio_sample_batch = load_fn!(
            b"retro_set_audio_sample_batch\0",
            unsafe extern "C" fn(RetroAudioSampleBatchFn)
        );
        let retro_set_input_poll = load_fn!(
            b"retro_set_input_poll\0",
            unsafe extern "C" fn(RetroInputPollFn)
        );
        let retro_set_input_state = load_fn!(
            b"retro_set_input_state\0",
            unsafe extern "C" fn(RetroInputStateFn)
        );
        let retro_load_game = load_fn!(
            b"retro_load_game\0",
            unsafe extern "C" fn(*const RetroGameInfo) -> bool
        );
        let retro_set_controller_port_device = load_fn!(
            b"retro_set_controller_port_device\0",
            unsafe extern "C" fn(port: u32, device: u32)
        );
        let retro_unload_game = load_fn!(b"retro_unload_game\0", unsafe extern "C" fn());
        let retro_run = load_fn!(b"retro_run\0", unsafe extern "C" fn());
        let retro_get_system_av_info = load_fn!(
            b"retro_get_system_av_info\0",
            unsafe extern "C" fn(*mut RetroSystemAvInfo)
        );
        // Build the shared state the callbacks will read/write.
        let frame_buffer = Arc::new(Mutex::new(FrameBuffer::new()));

        // Create the shared audio sample buffer. The libretro audio callbacks
        // will push into this; cpal drains it on its audio thread.
        // We create the buffer here (before retro_init) so it can be installed
        // into CoreCallbackState immediately. The AudioOutput (which opens the
        // sound device) is created later, after retro_get_system_av_info gives
        // us the actual sample rate the core will output at.
        let audio_buffer: Arc<Mutex<std::collections::VecDeque<f32>>> =
            Arc::new(Mutex::new(std::collections::VecDeque::with_capacity(8192)));

        // CString converts a Rust &str to a null-terminated C string.
        // We store it in CoreCallbackState so the *const c_char pointer we
        // hand to the core in GET_SYSTEM_DIRECTORY stays valid.
        let system_directory = CString::new(system_dir.to_string_lossy().as_ref())
            .map_err(|_| LibretroError::GameLoad("Invalid system dir path".into()))?;

        // ── Init sequence (order is critical) ────────────────────────────────

        // 1. Register environment_cb first — the core stores this pointer
        //    and uses it immediately when retro_init() is called.
        unsafe { retro_set_environment(callbacks::environment_cb) };

        // 2. Install callback state BEFORE retro_init(). The core calls
        //    environment_cb during init to query the system directory, pixel
        //    format etc. If we install state after, those calls hit None and
        //    the core gets back garbage or false for everything.
        callbacks::install_state(CoreCallbackState {
            frame_buffer: Arc::clone(&frame_buffer),
            input_state: Arc::clone(&input_state),
            pixel_format: RetroPixelFormat::Xrgb8888,
            system_directory,
            audio_buffer: Arc::clone(&audio_buffer),
        });

        // 3. Wake up the core — triggers environment_cb calls immediately.
        unsafe { retro_init() };

        // 4. Register the remaining callbacks. These are not called during
        //    init so order relative to step 3 doesn't strictly matter, but
        //    they must all be in place before retro_load_game().
        unsafe {
            retro_set_video_refresh(callbacks::video_refresh_cb);
            retro_set_audio_sample(callbacks::audio_sample_cb);
            retro_set_audio_sample_batch(callbacks::audio_sample_batch_cb);
            retro_set_input_poll(callbacks::input_poll_cb);
            retro_set_input_state(callbacks::input_state_cb);
        }

        // 5. Query system info to find out how the core wants its ROM delivered.
        //    MaybeUninit::uninit() allocates stack space for the struct without
        //    initialising it — no zeroing. We then pass a raw pointer to the C
        //    function which fills every field. assume_init() tells Rust "trust
        //    me, it's fully initialised now" — safe here because the libretro
        //    spec requires retro_get_system_info() to fill all fields.
        //    Using zeroed() would also work but wastes a memset.
        let system_info = unsafe {
            let mut info = std::mem::MaybeUninit::<RetroSystemInfo>::uninit();
            retro_get_system_info(info.as_mut_ptr());
            info.assume_init()
        };

        // The ROM path as a null-terminated C string. Must stay alive until
        // after retro_load_game() returns.
        let rom_cstring = CString::new(rom_path.to_string_lossy().as_ref())
            .map_err(|_| LibretroError::GameLoad("Invalid ROM path".into()))?;

        // 6. Build RetroGameInfo and load the ROM.
        //    need_fullpath = true  → pass the path, leave data null (fceumm).
        //    need_fullpath = false → read the file and pass a data pointer.
        //
        //    `_rom_data` holds the file bytes in memory for the need_fullpath=false
        //    case. RetroGameInfo.data is a raw pointer into this Vec, so the Vec
        //    must stay alive until retro_load_game() returns. Declaring it here
        //    (outside the if/else) ensures it lives long enough regardless of
        //    which branch we take. The underscore prefix silences the unused
        //    variable warning in the need_fullpath=true branch where it stays None.
        let _rom_data: Option<Vec<u8>>;
        let game_info = if system_info.need_fullpath {
            _rom_data = None;
            RetroGameInfo {
                path: rom_cstring.as_ptr(),
                data: std::ptr::null(),
                size: 0,
                meta: std::ptr::null(),
            }
        } else {
            // Read the whole ROM into memory and pass a pointer to the data.
            let data = std::fs::read(rom_path)
                .map_err(|e| LibretroError::GameLoad(format!("Failed to read ROM: {e}")))?;
            let size = data.len();
            _rom_data = Some(data);
            RetroGameInfo {
                path: rom_cstring.as_ptr(),
                data: _rom_data.as_ref().unwrap().as_ptr() as *const c_void,
                size,
                meta: std::ptr::null(),
            }
        };

        let loaded = unsafe { retro_load_game(&game_info) };
        if !loaded {
            // Clean up on failure so the global state doesn't leak.
            unsafe { retro_deinit() };
            callbacks::remove_state();
            return Err(LibretroError::GameLoad(
                "retro_load_game() returned false — check ROM path and core compatibility".into(),
            ));
        }

        unsafe {
            // Tell the core we're using a standard gamepad on port 1.
            retro_set_controller_port_device(0, 1); // port 1, device RETRO_DEVICE_JOYPAD
        }

        // 7. Read final output geometry and timing now that the game is loaded.
        //    The core may have called SET_GEOMETRY during load_game already,
        //    but get_system_av_info gives us the authoritative fps value.
        let av_info = unsafe {
            let mut info = std::mem::MaybeUninit::<RetroSystemAvInfo>::uninit();
            retro_get_system_av_info(info.as_mut_ptr());
            info.assume_init()
        };

        // Size the frame buffer to the reported base resolution.
        {
            let mut fb = frame_buffer.lock().expect("frame buffer lock");
            fb.width = av_info.geometry.base_width;
            fb.height = av_info.geometry.base_height;
            let len = (av_info.geometry.base_width * av_info.geometry.base_height * 4) as usize;
            fb.rgba_data.resize(len, 0);
        }

        // Now that we know the core's sample rate, open the audio output device.
        // We pass the same audio_buffer Arc the callbacks are already using, so
        // cpal will drain exactly the samples the core produces.
        // On failure (e.g. no sound card) we log a warning and continue silently.
        let sample_rate = av_info.timing.sample_rate as u32;
        let audio_output = match AudioOutput::new(sample_rate, Arc::clone(&audio_buffer)) {
            Ok(output) => Some(output),
            Err(e) => {
                // Non-fatal: the game runs silently if no sound device is available.
                tracing::warn!("Audio output unavailable: {e}");
                None
            }
        };

        Ok(Self {
            _library: lib,
            retro_run,
            retro_unload_game,
            retro_deinit,
            frame_buffer,
            input_state,
            _audio_output: audio_output,
            fps: av_info.timing.fps,
        })
    }

    /// Advance the emulation by one frame. Calls the core's retro_run(), which
    /// in turn calls our video_refresh_cb, audio callbacks, and input callbacks.
    ///
    /// Must be called from the same thread that called load() — the libretro
    /// spec requires retro_run() and retro_init() to be on the same thread.
    pub fn run_frame(&self) {
        unsafe { (self.retro_run)() }
    }

    /// Cleanly shut down the core and release all resources.
    ///
    /// Takes `self` by value so the compiler prevents any further calls to
    /// run_frame() after shutdown — the function pointers would dangle once
    /// _library is dropped at the end of this function.
    pub fn shutdown(self) {
        unsafe {
            (self.retro_unload_game)();
            (self.retro_deinit)();
        }
        callbacks::remove_state();
        // _library is dropped here → dlclose() is called → .so is unmapped.
    }
}
