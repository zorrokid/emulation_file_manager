pub const JOYPAD_B: u32 = 0;
pub const JOYPAD_Y: u32 = 1;
pub const JOYPAD_SELECT: u32 = 2;
pub const JOYPAD_START: u32 = 3;
pub const JOYPAD_UP: u32 = 4;
pub const JOYPAD_DOWN: u32 = 5;
pub const JOYPAD_LEFT: u32 = 6;
pub const JOYPAD_RIGHT: u32 = 7;
pub const JOYPAD_A: u32 = 8;
pub const JOYPAD_X: u32 = 9;
pub const JOYPAD_L: u32 = 10;
pub const JOYPAD_R: u32 = 11;

pub struct InputState {
    buttons: [bool; 16],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buttons: [false; 16],
        }
    }

    pub fn set_button(&mut self, id: u32, pressed: bool) {
        if let Some(slot) = self.buttons.get_mut(id as usize) {
            *slot = pressed;
        }
    }

    pub fn get_button(&self, id: u32) -> bool {
        self.buttons.get(id as usize).copied().unwrap_or(false)
    }
}
