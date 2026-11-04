/// HID keyboard usage IDs; values go straight into the report's key slots.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum HidSerialKeyboardKey {
    NONE = 0x00,
    // function keys
    ESC = 0x29,
    F1 = 0x3A,
    F2 = 0x3B,
    F3 = 0x3C,
    F4 = 0x3D,
    F5 = 0x3E,
}

/// Bits of the report's modifier byte.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum HidSerialKeyboardModifier {
    LCTRL = 0x01,
    LSHIFT = 0x02,
    LALT = 0x04,
    LGUI = 0x08,
    RCTRL = 0x10,
    RSHIFT = 0x20,
    RALT = 0x40,
    RGUI = 0x80,
}
