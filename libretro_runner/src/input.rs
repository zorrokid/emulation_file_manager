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

/// Frontend-owned shared input state exposed to libretro callbacks.
///
/// Digital button state is read through `RETRO_DEVICE_JOYPAD`, and analog
/// axis state is read through `RETRO_DEVICE_ANALOG`.
///
/// Out-of-range reads return safe defaults (`false` / `0`), and out-of-range
/// writes are ignored.
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
    /// Create a new input state with all buttons released and all analog axes centered.
    pub fn new() -> Self {
        Self {
            buttons: [false; 16],
            axes: [[0; 2]; 2],
        }
    }

    /// Set the pressed state for a libretro joypad button ID.
    ///
    /// Invalid button IDs are ignored.
    pub fn set_button(&mut self, id: u32, pressed: bool) {
        if let Some(slot) = self.buttons.get_mut(id as usize) {
            *slot = pressed;
        }
    }

    /// Get the pressed state for a libretro joypad button ID.
    ///
    /// Returns `false` for unknown button IDs.
    pub fn get_button(&self, id: u32) -> bool {
        self.buttons.get(id as usize).copied().unwrap_or(false)
    }

    /// Get the value of a libretro analog axis.
    ///
    /// `index` selects the stick and `id` selects the axis within that stick.
    /// Returns `0` for unknown stick or axis IDs.
    pub fn get_axis(&self, index: u32, id: u32) -> i16 {
        self.axes
            .get(index as usize)
            .and_then(|stick| stick.get(id as usize))
            .copied()
            .unwrap_or(0)
    }

    /// Set the value of a libretro analog axis.
    ///
    /// `index` selects the stick and `id` selects the axis within that stick.
    /// Invalid stick or axis IDs are ignored.
    pub fn set_axis(&mut self, index: u32, id: u32, value: i16) {
        if let Some(stick) = self.axes.get_mut(index as usize)
            && let Some(slot) = stick.get_mut(id as usize)
        {
            *slot = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_button_state() {
        let mut state = InputState::new();

        state.set_button(JOYPAD_A, true);
        state.set_button(JOYPAD_B, false);

        assert!(state.get_button(JOYPAD_A));
        assert!(!state.get_button(JOYPAD_B));
    }

    #[test]
    fn set_and_get_axis_state() {
        let mut state = InputState::new();

        state.set_axis(0, 0, 1234);
        state.set_axis(1, 1, -2345);

        assert_eq!(state.get_axis(0, 0), 1234);
        assert_eq!(state.get_axis(1, 1), -2345);
    }

    #[test]
    fn out_of_range_reads_return_safe_defaults() {
        let state = InputState::new();

        assert!(!state.get_button(99));
        assert_eq!(state.get_axis(99, 0), 0);
        assert_eq!(state.get_axis(0, 99), 0);
    }

    #[test]
    fn out_of_range_writes_are_ignored() {
        let mut state = InputState::new();

        state.set_button(JOYPAD_A, true);
        state.set_axis(0, 0, 1234);

        state.set_button(99, true);
        state.set_axis(99, 0, 9999);
        state.set_axis(0, 99, 9999);

        assert!(state.get_button(JOYPAD_A));
        assert_eq!(state.get_axis(0, 0), 1234);
        assert!(!state.get_button(99));
        assert_eq!(state.get_axis(99, 0), 0);
        assert_eq!(state.get_axis(0, 99), 0);
    }
}
