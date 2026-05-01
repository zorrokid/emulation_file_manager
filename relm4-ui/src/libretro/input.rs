use std::sync::{Arc, Mutex};

use gilrs::{Axis, EventType};
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

/// Libretro's axis range is [-32768, 32767]
const LIBRETRO_AXIS_MAX: i16 = 32767;

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

fn apply_button_event(button: gilrs::Button, pressed: bool, state: &Arc<Mutex<InputState>>) {
    let mut state = state.lock().expect("input state lock");
    if let Some(libretro_button) = button_to_libretro_button(button) {
        state.set_button(libretro_button, pressed);
    }
}

fn apply_axis_event(axis: Axis, value: f32, state: &Arc<Mutex<InputState>>) {
    let mut state = state.lock().expect("input state lock");
    let libretro_value = scale_gilrs_axis_to_libretro_axis(value, axis);
    match axis {
        gilrs::Axis::LeftStickX => state.set_axis(0, 0, libretro_value),
        gilrs::Axis::LeftStickY => state.set_axis(0, 1, -libretro_value),
        gilrs::Axis::RightStickX => state.set_axis(1, 0, libretro_value),
        gilrs::Axis::RightStickY => state.set_axis(1, 1, -libretro_value),
        _ => {}
    }
}

pub fn map_gamepad_event(
    event_type: EventType,
    state: Arc<Mutex<InputState>>,
    _input_profile: Arc<Mutex<InputProfile>>,
) {
    match event_type {
        EventType::ButtonPressed(button, _code) => {
            apply_button_event(button, true, &state);
        }
        EventType::ButtonReleased(button, _code) => {
            apply_button_event(button, false, &state);
        }
        EventType::AxisChanged(axis, value, _code) => {
            apply_axis_event(axis, value, &state);
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
/// Also invert y-axis.
fn scale_gilrs_axis_to_libretro_axis(value: f32, axis: gilrs::Axis) -> i16 {
    // TODO: is this good value for a deadzone? We want to ignore small inputs near the center to
    // prevent drift.
    const DEADZONE: f32 = 0.1;
    if value.abs() < DEADZONE {
        return 0;
    }
    let axis_invert = if axis == gilrs::Axis::LeftStickY || axis == gilrs::Axis::RightStickY {
        -1.0
    } else {
        1.0
    };

    (value * LIBRETRO_AXIS_MAX as f32 * axis_invert).round() as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use gilrs::Button;
    use libretro_runner::input::InputState;
    use relm4::gtk;

    #[test]
    fn test_apply_axis_event() {
        let cases = [
            (gilrs::Axis::LeftStickX, 1.0, LIBRETRO_AXIS_MAX, 0, 0),
            (gilrs::Axis::LeftStickX, -1.0, -LIBRETRO_AXIS_MAX, 0, 0),
            (gilrs::Axis::LeftStickY, 1.0, LIBRETRO_AXIS_MAX, 0, 1),
            (gilrs::Axis::LeftStickY, -1.0, -LIBRETRO_AXIS_MAX, 0, 1),
            (gilrs::Axis::RightStickX, 1.0, LIBRETRO_AXIS_MAX, 1, 0),
            (gilrs::Axis::RightStickX, -1.0, -LIBRETRO_AXIS_MAX, 1, 0),
            (gilrs::Axis::RightStickY, 1.0, LIBRETRO_AXIS_MAX, 1, 1),
            (gilrs::Axis::RightStickY, -1.0, -LIBRETRO_AXIS_MAX, 1, 1),
        ];
        let state = Arc::new(Mutex::new(InputState::default()));
        for (axis, value, expected_value, stick_index, axis_index) in cases {
            apply_axis_event(axis, value, &state);
            {
                let state = state.lock().unwrap();
                assert_eq!(state.get_axis(stick_index, axis_index), expected_value);
            }
        }
    }

    #[test]
    fn test_apply_axis_event_movement_in_dead_zone() {
        let state = Arc::new(Mutex::new(InputState::default()));
        apply_axis_event(gilrs::Axis::LeftStickX, 0.01, &state);
        let state = state.lock().unwrap();
        assert_eq!(state.get_axis(0, 0), 0);
    }

    #[test]
    fn test_apply_button_event() {
        let cases = [
            (Button::South, true, JOYPAD_A),
            (Button::South, false, JOYPAD_A),
            (Button::North, true, JOYPAD_Y),
            (Button::North, false, JOYPAD_Y),
            (Button::West, true, JOYPAD_X),
            (Button::West, false, JOYPAD_X),
            (Button::East, true, JOYPAD_B),
            (Button::East, false, JOYPAD_B),
            (Button::LeftTrigger, true, JOYPAD_L),
            (Button::LeftTrigger, false, JOYPAD_L),
            (Button::RightTrigger, true, JOYPAD_R),
            (Button::RightTrigger, false, JOYPAD_R),
            (Button::Select, true, JOYPAD_SELECT),
            (Button::Select, false, JOYPAD_SELECT),
            (Button::Start, true, JOYPAD_START),
            (Button::Start, false, JOYPAD_START),
            (Button::DPadUp, true, JOYPAD_UP),
            (Button::DPadUp, false, JOYPAD_UP),
            (Button::DPadDown, true, JOYPAD_DOWN),
            (Button::DPadDown, false, JOYPAD_DOWN),
            (Button::DPadLeft, true, JOYPAD_LEFT),
            (Button::DPadLeft, false, JOYPAD_LEFT),
            (Button::DPadRight, true, JOYPAD_RIGHT),
            (Button::DPadRight, false, JOYPAD_RIGHT),
        ];
        let state = Arc::new(Mutex::new(InputState::default()));
        for (button, pressed, libretro_button) in cases {
            apply_button_event(button, pressed, &state);
            {
                let state = state.lock().unwrap();
                assert!(state.get_button(libretro_button) == pressed);
            }
        }
    }

    #[test]
    fn test_map_key_events_maps_supported_keys() {
        let cases = [
            (gtk::gdk::Key::Down, JOYPAD_DOWN),
            (gtk::gdk::Key::Up, JOYPAD_UP),
            (gtk::gdk::Key::z, JOYPAD_B),
            (gtk::gdk::Key::Z, JOYPAD_B),
            (gtk::gdk::Key::x, JOYPAD_A),
            (gtk::gdk::Key::X, JOYPAD_A),
            (gtk::gdk::Key::a, JOYPAD_Y),
            (gtk::gdk::Key::A, JOYPAD_Y),
            (gtk::gdk::Key::s, JOYPAD_X),
            (gtk::gdk::Key::S, JOYPAD_X),
            (gtk::gdk::Key::q, JOYPAD_L),
            (gtk::gdk::Key::Q, JOYPAD_L),
            (gtk::gdk::Key::w, JOYPAD_R),
            (gtk::gdk::Key::W, JOYPAD_R),
            (gtk::gdk::Key::Return, JOYPAD_START),
            (gtk::gdk::Key::BackSpace, JOYPAD_SELECT),
        ];

        let input_state = Arc::new(Mutex::new(InputState::default()));

        for (key, expected_button) in cases {
            map_key_event(key, &input_state, true);
            let state = input_state.lock().unwrap();
            assert!(state.get_button(expected_button));
        }
    }

    #[test]
    fn test_map_key_events_release() {
        let input_state = Arc::new(Mutex::new(InputState::default()));
        map_key_event(gtk::gdk::Key::x, &input_state, true);
        {
            let state = input_state.lock().unwrap();
            assert!(state.get_button(JOYPAD_A));
        }
        map_key_event(gtk::gdk::Key::x, &input_state, false);
        {
            let state = input_state.lock().unwrap();
            assert!(!state.get_button(JOYPAD_A));
        }
    }
}
