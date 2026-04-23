// std::os::raw provides Rust equivalents of C primitive types.
// c_char is i8 (C's `char`), c_void is the Rust stand-in for C's `void`.
// We need these so our struct fields and pointer types match what the C code expects.
use std::os::raw::{c_char, c_void};

// Commands the core sends to the frontend via environment_cb.
// These are integer IDs defined in libretro.h — we only declare the ones we handle.
pub const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: u32 = 9;
pub const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;
pub const RETRO_ENVIRONMENT_GET_VARIABLE: u32 = 15;
pub const RETRO_ENVIRONMENT_GET_LOG_INTERFACE: u32 = 27;
pub const RETRO_ENVIRONMENT_SET_GEOMETRY: u32 = 37;

// Input device IDs used by retro_input_state_t callbacks.
pub const RETRO_DEVICE_JOYPAD: u32 = 1;
pub const RETRO_DEVICE_ANALOG: u32 = 5;

// #[repr(u32)] tells Rust to store this enum as a plain u32 in memory,
// matching the C enum type. Without this the size and layout would be
// undefined and casting a raw u32 from C into this type would be unsafe.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetroPixelFormat {
    Rgb1555 = 0,   // legacy default
    Xrgb8888 = 1,  // preferred — we accept this
    Rgb565 = 2,
}

// #[repr(C)] on all structs below guarantees that field order and padding
// match what the C compiler produces. Without it Rust can reorder fields
// freely, which would cause us to read the wrong bytes when we cast a raw
// pointer from the C library into one of these types.

/// Passed to retro_load_game(). Tells the core where the ROM is.
/// `path` and `data` are mutually exclusive — fceumm uses `path`
/// (need_fullpath = true) so we set `data` to null and `size` to 0.
#[repr(C)]
pub struct RetroGameInfo {
    // *const c_char is a raw pointer to a null-terminated C string.
    // Equivalent to `const char*` in C.
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

/// Filled by retro_get_system_info(). We read need_fullpath before loading.
#[repr(C)]
pub struct RetroSystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    /// If true, pass the ROM file path in RetroGameInfo.path instead of
    /// loading the file into memory and passing a data pointer.
    pub need_fullpath: bool,
    pub block_extract: bool,
}

/// Output resolution and aspect ratio reported by the core.
#[repr(C)]
pub struct RetroGameGeometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

/// Timing information reported by the core after retro_load_game().
#[repr(C)]
pub struct RetroSystemTiming {
    pub fps: f64,         // ~60.0988 for NTSC NES
    pub sample_rate: f64, // audio sample rate — used to configure the cpal output stream
}

/// Filled by retro_get_system_av_info(). We read fps and geometry from here
/// after the game is loaded to set up the game loop interval and frame buffer.
#[repr(C)]
pub struct RetroSystemAvInfo {
    pub geometry: RetroGameGeometry,
    pub timing: RetroSystemTiming,
}

// Function pointer type aliases for the callbacks we register with the core.
//
// `unsafe extern "C"` means:
//   - `extern "C"`: use the C calling convention (how arguments are passed in
//     registers/stack). Required so the C core can call our Rust functions.
//   - `unsafe`: the caller (the C core) cannot uphold Rust's safety guarantees,
//     so any call through these pointers is inherently unsafe.
//
// These types are used when calling retro_set_video_refresh() etc. to hand
// our Rust callback functions to the core as C-compatible function pointers.
pub type RetroEnvironmentFn =
    unsafe extern "C" fn(cmd: u32, data: *mut c_void) -> bool;
pub type RetroVideoRefreshFn =
    unsafe extern "C" fn(data: *const c_void, width: u32, height: u32, pitch: usize);
pub type RetroAudioSampleFn =
    unsafe extern "C" fn(left: i16, right: i16);
pub type RetroAudioSampleBatchFn =
    unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
pub type RetroInputPollFn =
    unsafe extern "C" fn();
pub type RetroInputStateFn =
    unsafe extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16;
