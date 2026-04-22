// See for button definitions:
// https://github.com/libretro/libretro-common/blob/master/include/libretro.h
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
    // Standard buttons (0-15)
    buttons: [bool; 16],
    // [Stick Index (Left=0, Right=1)][Axis ID (X=0, Y=1)]
    axes: [[i16; 2]; 2],
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buttons: [false; 16],
            axes: [[0; 2]; 2],
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

    pub fn get_axis(&self, index: u32, id: u32) -> i16 {
        self.axes
            .get(index as usize)
            .and_then(|stick| stick.get(id as usize))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_axis(&mut self, index: u32, id: u32, value: i16) {
        if let Some(stick) = self.axes.get_mut(index as usize)
            && let Some(slot) = stick.get_mut(id as usize)
        {
            *slot = value;
        }
    }
}
