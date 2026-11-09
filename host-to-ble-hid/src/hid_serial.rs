use tokio::io::AsyncWriteExt;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::hid_serial::hid_serial_keyboard::HidSerialKeyboardReport;
use crate::hid_serial::hid_serial_mouse::HidSerialMouseReport;
use crate::hid_serial::hid_serial_type::{HID_SERIAL_BAUD_RATE, HID_SERIAL_SOF, HidSerialType};

pub mod hid_serial_keyboard;
pub mod hid_serial_mouse;
pub mod hid_serial_type;

pub struct HidSerialDeviceInfo {
    pub port_name: String,
}

pub struct HidSerialHandler {
    serial_port: SerialStream,
}

impl HidSerialHandler {
    pub fn scan() -> Result<Vec<HidSerialDeviceInfo>, tokio_serial::Error> {
        let infos = tokio_serial::available_ports()?;
        Ok(infos
            .into_iter()
            .map(|info| HidSerialDeviceInfo {
                port_name: info.port_name,
            })
            .collect())
    }

    pub fn connect(device_info: &HidSerialDeviceInfo) -> Result<Self, tokio_serial::Error> {
        let serial_port =
            tokio_serial::new(&device_info.port_name, HID_SERIAL_BAUD_RATE).open_native_async()?;
        Ok(Self { serial_port })
    }

    /// Writes one `SOF | type | len | payload | checksum` frame.
    async fn send(&mut self, hid_type: HidSerialType, payload: &[u8]) -> std::io::Result<()> {
        let len = payload.len() as u8;
        let checksum = payload.iter().fold(hid_type as u8 ^ len, |sum, b| sum ^ b);

        let mut frame = vec![HID_SERIAL_SOF, hid_type as u8, len];
        frame.extend_from_slice(payload);
        frame.push(checksum);

        self.serial_port.write_all(&frame).await
    }

    pub async fn send_keyboard(&mut self, report: &HidSerialKeyboardReport) -> std::io::Result<()> {
        self.send(HidSerialType::KEYBOARD, &report.to_bytes()).await
    }

    pub async fn send_mouse(&mut self, report: &HidSerialMouseReport) -> std::io::Result<()> {
        self.send(HidSerialType::MOUSE, &report.to_bytes()).await
    }
}
