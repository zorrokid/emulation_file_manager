use std::sync::{Arc, Mutex};

use gilrs::EventType;
use relm4::gtk;

use libretro_runner::{
    input::{
        InputState, JOYPAD_A, JOYPAD_B, JOYPAD_DOWN, JOYPAD_L, JOYPAD_LEFT, JOYPAD_R, JOYPAD_RIGHT,
        JOYPAD_SELECT, JOYPAD_START, JOYPAD_UP, JOYPAD_X, JOYPAD_Y,
    },
    supported_cores::InputProfile,
};

// Axis values beyond this will be treated as fully pressed in
// that direction.
// const SNAP_THRESHOLD: i16 = 2000;

/// Map a GTK key value to a libretro JOYPAD button ID, if we handle it.
fn keyval_to_button(keyval: gtk::gdk::Key) -> Option<u32> {
    match keyval {
        gtk::gdk::Key::Up => Some(JOYPAD_UP),
        gtk::gdk::Key::Down => Some(JOYPAD_DOWN),
        gtk::gdk::Key::Left => Some(JOYPAD_LEFT),
        gtk::gdk::Key::Right => Some(JOYPAD_RIGHT),
        // Z = B, X = A, A = Y, S = X — common retro gaming convention.
        gtk::gdk::Key::z | gtk::gdk::Key::Z => Some(JOYPAD_B),
        gtk::gdk::Key::x | gtk::gdk::Key::X => Some(JOYPAD_A),
        gtk::gdk::Key::a | gtk::gdk::Key::A => Some(JOYPAD_Y),
        gtk::gdk::Key::s | gtk::gdk::Key::S => Some(JOYPAD_X),
        gtk::gdk::Key::q | gtk::gdk::Key::Q => Some(JOYPAD_L),
        gtk::gdk::Key::w | gtk::gdk::Key::W => Some(JOYPAD_R),
        gtk::gdk::Key::Return => Some(JOYPAD_START),
        gtk::gdk::Key::BackSpace => Some(JOYPAD_SELECT),
        _ => None,
    }
}

/// Called from the EventControllerKey handlers on the game window.
/// `pressed` is true for key-down events and false for key-up.
pub fn map_key_event(keyval: gtk::gdk::Key, input_state: &Arc<Mutex<InputState>>, pressed: bool) {
    if let Some(button) = keyval_to_button(keyval) {
        input_state
            .lock()
            .expect("input state lock")
            .set_button(button, pressed);
    }
}

pub fn map_gamepad_event(
    event_type: EventType,
    input_state: &Arc<Mutex<InputState>>,
    input_profile: &Arc<Mutex<InputProfile>>,
) {
    let mut state = input_state.lock().expect("input state lock");
    match event_type {
        EventType::ButtonPressed(button, _code) => {
            if let Some(libretro_button) = button_to_libretro_button(button) {
                state.set_button(libretro_button, true);
            }
        }
        EventType::ButtonReleased(button, _code) => {
            if let Some(libretro_button) = button_to_libretro_button(button) {
                state.set_button(libretro_button, false);
            }
        }
        EventType::AxisChanged(axis, value, _code) => {
            let libretro_value = scale_gilrs_axis_to_libretro_axis(value);

            let input_profile_guard = input_profile.lock();
            let input_profile = match input_profile_guard {
                Ok(profile) => profile.clone(),
                Err(_) => {
                    tracing::error!(
                        "Failed to lock input profile mutex, defaulting to Standard profile"
                    );
                    InputProfile::Standard
                }
            };
            match (axis, input_profile) {
                (gilrs::Axis::LeftStickX, _) => state.set_axis(0, 0, libretro_value),
                (gilrs::Axis::LeftStickY, _) => state.set_axis(0, 1, -libretro_value),
                (gilrs::Axis::RightStickX, _) => state.set_axis(1, 0, libretro_value),
                (gilrs::Axis::RightStickY, _) => state.set_axis(1, 1, -libretro_value),
                /*(gilrs::Axis::RightStickX, InputProfile::Intellivision) => {
                    state.set_button(INT_KEY_2, libretro_value < -SNAP_THRESHOLD);
                    state.set_button(INT_KEY_8, libretro_value > SNAP_THRESHOLD);
                }
                (gilrs::Axis::RightStickY, InputProfile::Intellivision) => {
                    state.set_button(INT_KEY_4, libretro_value < -SNAP_THRESHOLD);
                    state.set_button(INT_KEY_6, libretro_value > SNAP_THRESHOLD);
                }*/
                _ => {}
            }
        }
        _ => {}
    }
}

fn button_to_libretro_button(button: gilrs::Button) -> Option<u32> {
    use gilrs::Button::*;
    match button {
        South => Some(JOYPAD_A),
        East => Some(JOYPAD_B),
        North => Some(JOYPAD_Y),
        West => Some(JOYPAD_X),
        LeftTrigger => Some(JOYPAD_L),
        RightTrigger => Some(JOYPAD_R),
        Select => Some(JOYPAD_SELECT),
        Start => Some(JOYPAD_START),
        DPadUp => Some(JOYPAD_UP),
        DPadDown => Some(JOYPAD_DOWN),
        DPadLeft => Some(JOYPAD_LEFT),
        DPadRight => Some(JOYPAD_RIGHT),
        _ => None,
    }
}

/// Gilrs axes are in the range [-1.0, 1.0]. Scale to libretro's expected
/// range of [-32768, 32767]. Also apply a deadzone to ignore small inputs near the center.
fn scale_gilrs_axis_to_libretro_axis(value: f32) -> i16 {
    const LIBRETRO_AXIS_MAX: f32 = 32767.0;
    // TODO: is this good value for a deadzone? We want to ignore small inputs near the center to
    // prevent drift.
    const DEADZONE: f32 = 0.1;
    if value.abs() < DEADZONE {
        return 0;
    }
    (value * LIBRETRO_AXIS_MAX).round() as i16
}
