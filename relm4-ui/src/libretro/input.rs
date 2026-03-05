use std::sync::{Arc, Mutex};

use relm4::gtk;

use libretro_runner::input::{
    InputState, JOYPAD_A, JOYPAD_B, JOYPAD_DOWN, JOYPAD_L, JOYPAD_LEFT, JOYPAD_RIGHT,
    JOYPAD_SELECT, JOYPAD_START, JOYPAD_UP, JOYPAD_X, JOYPAD_Y, JOYPAD_R,
};

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
