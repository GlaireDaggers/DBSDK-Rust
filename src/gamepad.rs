use bitmask::bitmask;

use crate::db_internal::{gamepad_readState, gamepad_setRumble, gamepad_isConnected};

#[repr(C)]
#[derive(Clone, Copy)]
pub enum GamepadSlot {
    SlotA,
    SlotB,
    SlotC,
    SlotD,
}

bitmask! {
    #[repr(C)]
    pub mask GamepadButtonMask: u16 where flags GamepadButton {
        A       = 1,
        B       = (1 << 1),
        X       = (1 << 2),
        Y       = (1 << 3),
        Up      = (1 << 4),
        Down    = (1 << 5),
        Left    = (1 << 6),
        Right   = (1 << 7),
        L1      = (1 << 8),
        L2      = (1 << 9),
        L3      = (1 << 10),
        R1      = (1 << 11),
        R2      = (1 << 12),
        R3      = (1 << 13),
        Select  = (1 << 14),
        Start   = (1 << 15),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GamepadState {
    pub button_mask: GamepadButtonMask,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
}

impl GamepadState {
    /// Check if the given button is pressed
    pub fn is_pressed(self, button: GamepadButton) -> bool {
        return self.button_mask.contains(button);
    }
}

pub struct Gamepad {
    pub slot: GamepadSlot,
}

impl Gamepad {
    /// Construct a new Gamepad for the given slot
    pub const fn new(slot: GamepadSlot) -> Gamepad {
        return Gamepad { slot: slot };
    }

    /// Check whether this gamepad is connected
    pub fn is_connected(&self) -> bool {
        unsafe { return gamepad_isConnected(self.slot); }
    }

    /// Read the state of this gamepad
    pub fn read_state(&self) -> GamepadState {
        let mut state = GamepadState { button_mask: GamepadButtonMask::none(), left_stick_x: 0, left_stick_y: 0, right_stick_x: 0, right_stick_y: 0 };
        unsafe { gamepad_readState(self.slot, &mut state); }
        return state;
    }

    /// Set this gamepad's vibration on or off
    pub fn set_rumble(&self, enable: bool) {
        unsafe { gamepad_setRumble(self.slot, enable); }
    }
}