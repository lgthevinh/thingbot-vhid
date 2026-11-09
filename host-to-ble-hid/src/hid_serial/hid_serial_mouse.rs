use evdev::KeyCode;

use crate::hid_serial::hid_serial_type::HID_SERIAL_MOUSE_LEN;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct HidSerialMouseReport {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
}

impl HidSerialMouseReport {
    pub fn to_bytes(&self) -> [u8; HID_SERIAL_MOUSE_LEN] {
        [self.buttons, self.x as u8, self.y as u8, self.wheel as u8]
    }
}

/// Bit of the report's button byte for a mouse button.
pub fn button_bit(key: KeyCode) -> Option<u8> {
    match key {
        KeyCode::BTN_LEFT => Some(0x01),
        KeyCode::BTN_RIGHT => Some(0x02),
        KeyCode::BTN_MIDDLE => Some(0x04),
        _ => None,
    }
}

/// Splits accumulated deltas into reports that each fit the firmware's -127..=127 range.
pub fn split_motion(
    buttons: u8,
    mut x: i32,
    mut y: i32,
    mut wheel: i32,
) -> Vec<HidSerialMouseReport> {
    let take = |v: &mut i32| {
        let part = (*v).clamp(-127, 127);
        *v -= part;
        part as i8
    };

    let mut reports = Vec::new();
    loop {
        reports.push(HidSerialMouseReport {
            buttons,
            x: take(&mut x),
            y: take(&mut y),
            wheel: take(&mut wheel),
        });
        if x == 0 && y == 0 && wheel == 0 {
            return reports;
        }
    }
}
