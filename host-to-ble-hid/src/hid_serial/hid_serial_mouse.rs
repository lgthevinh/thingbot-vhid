/// Bits of the report's button byte.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum HidSerialMouseButton {
    LEFT = 0x01,
    RIGHT = 0x02,
    MIDDLE = 0x04,
}
