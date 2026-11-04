/// Sync byte marking the start of every serial HID frame.
pub const HID_SERIAL_SOF: u8 = 0xAA;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum HidSerialType {
    KEYBOARD = 0x01,
    MOUSE = 0x02,
}

pub const HID_SERIAL_BAUD_RATE: u32 = 115200;
pub const HID_SERIAL_KEYBOARD_LEN: usize = 8;
pub const HID_SERIAL_MOUSE_LEN: usize = 4;
