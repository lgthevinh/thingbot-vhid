use evdev::KeyCode;

use crate::hid_serial::hid_serial_type::HID_SERIAL_KEYBOARD_LEN;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct HidSerialKeyboardReport {
    pub modifiers: u8,
    pub keys: [u8; 6],
}

impl HidSerialKeyboardReport {
    /// Builds a report from the currently pressed keys; keys beyond the 6th are dropped.
    pub fn from_pressed<'a>(pressed: impl IntoIterator<Item = &'a KeyCode>) -> Self {
        let mut report = Self::default();
        let mut slot = 0;
        for &key in pressed {
            if let Some(bit) = modifier_bit(key) {
                report.modifiers |= bit;
            } else if let Some(usage) = usage(key)
                && slot < report.keys.len()
            {
                report.keys[slot] = usage;
                slot += 1;
            }
        }
        report
    }

    pub fn to_bytes(&self) -> [u8; HID_SERIAL_KEYBOARD_LEN] {
        let k = self.keys;
        [self.modifiers, 0, k[0], k[1], k[2], k[3], k[4], k[5]]
    }
}

/// Bit of the report's modifier byte for a modifier key.
fn modifier_bit(key: KeyCode) -> Option<u8> {
    Some(match key {
        KeyCode::KEY_LEFTCTRL => 0x01,
        KeyCode::KEY_LEFTSHIFT => 0x02,
        KeyCode::KEY_LEFTALT => 0x04,
        KeyCode::KEY_LEFTMETA => 0x08,
        KeyCode::KEY_RIGHTCTRL => 0x10,
        KeyCode::KEY_RIGHTSHIFT => 0x20,
        KeyCode::KEY_RIGHTALT => 0x40,
        KeyCode::KEY_RIGHTMETA => 0x80,
        _ => return None,
    })
}

/// HID keyboard usage ID for a Linux key code, limited to the firmware's 0x00..=0x65 range.
fn usage(key: KeyCode) -> Option<u8> {
    Some(match key {
        KeyCode::KEY_A => 0x04,
        KeyCode::KEY_B => 0x05,
        KeyCode::KEY_C => 0x06,
        KeyCode::KEY_D => 0x07,
        KeyCode::KEY_E => 0x08,
        KeyCode::KEY_F => 0x09,
        KeyCode::KEY_G => 0x0A,
        KeyCode::KEY_H => 0x0B,
        KeyCode::KEY_I => 0x0C,
        KeyCode::KEY_J => 0x0D,
        KeyCode::KEY_K => 0x0E,
        KeyCode::KEY_L => 0x0F,
        KeyCode::KEY_M => 0x10,
        KeyCode::KEY_N => 0x11,
        KeyCode::KEY_O => 0x12,
        KeyCode::KEY_P => 0x13,
        KeyCode::KEY_Q => 0x14,
        KeyCode::KEY_R => 0x15,
        KeyCode::KEY_S => 0x16,
        KeyCode::KEY_T => 0x17,
        KeyCode::KEY_U => 0x18,
        KeyCode::KEY_V => 0x19,
        KeyCode::KEY_W => 0x1A,
        KeyCode::KEY_X => 0x1B,
        KeyCode::KEY_Y => 0x1C,
        KeyCode::KEY_Z => 0x1D,
        KeyCode::KEY_1 => 0x1E,
        KeyCode::KEY_2 => 0x1F,
        KeyCode::KEY_3 => 0x20,
        KeyCode::KEY_4 => 0x21,
        KeyCode::KEY_5 => 0x22,
        KeyCode::KEY_6 => 0x23,
        KeyCode::KEY_7 => 0x24,
        KeyCode::KEY_8 => 0x25,
        KeyCode::KEY_9 => 0x26,
        KeyCode::KEY_0 => 0x27,
        KeyCode::KEY_ENTER => 0x28,
        KeyCode::KEY_ESC => 0x29,
        KeyCode::KEY_BACKSPACE => 0x2A,
        KeyCode::KEY_TAB => 0x2B,
        KeyCode::KEY_SPACE => 0x2C,
        KeyCode::KEY_MINUS => 0x2D,
        KeyCode::KEY_EQUAL => 0x2E,
        KeyCode::KEY_LEFTBRACE => 0x2F,
        KeyCode::KEY_RIGHTBRACE => 0x30,
        KeyCode::KEY_BACKSLASH => 0x31,
        KeyCode::KEY_SEMICOLON => 0x33,
        KeyCode::KEY_APOSTROPHE => 0x34,
        KeyCode::KEY_GRAVE => 0x35,
        KeyCode::KEY_COMMA => 0x36,
        KeyCode::KEY_DOT => 0x37,
        KeyCode::KEY_SLASH => 0x38,
        KeyCode::KEY_CAPSLOCK => 0x39,
        KeyCode::KEY_F1 => 0x3A,
        KeyCode::KEY_F2 => 0x3B,
        KeyCode::KEY_F3 => 0x3C,
        KeyCode::KEY_F4 => 0x3D,
        KeyCode::KEY_F5 => 0x3E,
        KeyCode::KEY_F6 => 0x3F,
        KeyCode::KEY_F7 => 0x40,
        KeyCode::KEY_F8 => 0x41,
        KeyCode::KEY_F9 => 0x42,
        KeyCode::KEY_F10 => 0x43,
        KeyCode::KEY_F11 => 0x44,
        KeyCode::KEY_F12 => 0x45,
        KeyCode::KEY_SYSRQ => 0x46,
        KeyCode::KEY_SCROLLLOCK => 0x47,
        KeyCode::KEY_PAUSE => 0x48,
        KeyCode::KEY_INSERT => 0x49,
        KeyCode::KEY_HOME => 0x4A,
        KeyCode::KEY_PAGEUP => 0x4B,
        KeyCode::KEY_DELETE => 0x4C,
        KeyCode::KEY_END => 0x4D,
        KeyCode::KEY_PAGEDOWN => 0x4E,
        KeyCode::KEY_RIGHT => 0x4F,
        KeyCode::KEY_LEFT => 0x50,
        KeyCode::KEY_DOWN => 0x51,
        KeyCode::KEY_UP => 0x52,
        KeyCode::KEY_NUMLOCK => 0x53,
        KeyCode::KEY_KPSLASH => 0x54,
        KeyCode::KEY_KPASTERISK => 0x55,
        KeyCode::KEY_KPMINUS => 0x56,
        KeyCode::KEY_KPPLUS => 0x57,
        KeyCode::KEY_KPENTER => 0x58,
        KeyCode::KEY_KP1 => 0x59,
        KeyCode::KEY_KP2 => 0x5A,
        KeyCode::KEY_KP3 => 0x5B,
        KeyCode::KEY_KP4 => 0x5C,
        KeyCode::KEY_KP5 => 0x5D,
        KeyCode::KEY_KP6 => 0x5E,
        KeyCode::KEY_KP7 => 0x5F,
        KeyCode::KEY_KP8 => 0x60,
        KeyCode::KEY_KP9 => 0x61,
        KeyCode::KEY_KP0 => 0x62,
        KeyCode::KEY_KPDOT => 0x63,
        KeyCode::KEY_102ND => 0x64,
        KeyCode::KEY_COMPOSE => 0x65,
        _ => return None,
    })
}
