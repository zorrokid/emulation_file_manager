// These tests require a real fceumm_libretro.so and a NES ROM on disk.
// They are marked #[ignore] so they don't run in CI where those files
// aren't available. To run them locally:
//
//   cargo test -p libretro_runner -- --ignored --test-threads=1
//
// --test-threads=1 is required because all three tests share the process-wide
// CORE_STATE global — running them in parallel would cause them to interfere.
//
// Set TEST_NES_ROM to point at any valid .nes ROM file:
//   TEST_NES_ROM=/path/to/game.nes cargo test -p libretro_runner -- --ignored --test-threads=1

use std::path::PathBuf;

use libretro_runner::{callbacks, core::LibretroCore};

fn core_path() -> PathBuf {
    PathBuf::from("/usr/lib/libretro/fceumm_libretro.so")
}

fn rom_path() -> PathBuf {
    // Allow overriding via environment variable so different machines can
    // point at their own ROM without changing the source.
    std::env::var("TEST_NES_ROM")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/test.nes"))
}

fn system_dir() -> PathBuf {
    PathBuf::from("/tmp")
}

/// After running 60 frames the video callback should have fired at least once,
/// setting frame_buffer.dirty = true.
#[test]
#[ignore = "requires /usr/lib/libretro/fceumm_libretro.so and TEST_NES_ROM"]
fn test_load_and_run_frames() {
    let core = LibretroCore::load(&core_path(), &rom_path(), &system_dir())
        .expect("LibretroCore::load failed");

    for _ in 0..60 {
        core.run_frame();
    }

    let dirty = core
        .frame_buffer
        .lock()
        .expect("frame buffer lock")
        .dirty;

    core.shutdown();

    assert!(dirty, "frame buffer should be dirty after running 60 frames");
}

/// The core reports its output resolution via retro_get_system_av_info()
/// after load and via SET_GEOMETRY during load. Either way the frame buffer
/// should have non-zero dimensions after the first frame.
#[test]
#[ignore = "requires /usr/lib/libretro/fceumm_libretro.so and TEST_NES_ROM"]
fn test_frame_buffer_dimensions() {
    let core = LibretroCore::load(&core_path(), &rom_path(), &system_dir())
        .expect("LibretroCore::load failed");

    core.run_frame();

    let (width, height) = {
        let fb = core.frame_buffer.lock().expect("frame buffer lock");
        (fb.width, fb.height)
    };

    core.shutdown();

    assert!(width > 0, "frame buffer width should be non-zero");
    assert!(height > 0, "frame buffer height should be non-zero");
}

/// shutdown() must clear CORE_STATE so a subsequent load can install fresh state.
/// We verify this by checking that with_state() returns None afterwards.
#[test]
#[ignore = "requires /usr/lib/libretro/fceumm_libretro.so and TEST_NES_ROM"]
fn test_shutdown_clears_state() {
    let core = LibretroCore::load(&core_path(), &rom_path(), &system_dir())
        .expect("LibretroCore::load failed");

    core.shutdown();

    let result = callbacks::with_state(|_| ());
    assert!(
        result.is_none(),
        "CORE_STATE should be None after shutdown"
    );
}
