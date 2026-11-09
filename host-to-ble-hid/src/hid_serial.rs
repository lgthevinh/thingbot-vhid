use tokio_serial::SerialPort;

use crate::hid_serial::hid_serial_type::HID_SERIAL_BAUD_RATE;

pub mod hid_serial_keyboard;
pub mod hid_serial_mouse;
pub mod hid_serial_type;

const TAG: &str = "hid_serial";
pub struct HidSerialDeviceInfo {
    port_name: String,
}

pub struct HidSerialPacket {
    _type: u8,
    len: u8,
    payload: Vec<u8>,
    checksum: u8,
}
pub struct HidSerialHandler {
    serial_port: Box<dyn SerialPort>,
}

impl HidSerialHandler {
    pub async fn scan() -> Result<Vec<HidSerialDeviceInfo>, tokio_serial::Error> {
        let mut device_infos = Vec::new();
        match tokio_serial::available_ports() {
            Ok(infos) => {
                for info in infos {
                    device_infos.push(HidSerialDeviceInfo {
                        port_name: info.port_name,
                    });
                }
                Ok(device_infos)
            }
            Err(e) => {
                tracing::debug!(TAG, "scan failed: {}", e);
                Err(e)
            }
        }
    }

    pub async fn connect(
        &mut self,
        device_info: HidSerialDeviceInfo,
    ) -> Result<(), tokio_serial::Error> {
        let serial_port = tokio_serial::new(device_info.port_name, HID_SERIAL_BAUD_RATE);
        self.serial_port = serial_port.open().unwrap();
        Ok(())
    }

    async fn send(&mut self, packet: HidSerialPacket) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes: Vec<u8> = Vec::new();

        bytes.push(packet._type);
        bytes.push(packet.len);
        for byte in packet.payload.into_iter() {
            bytes.push(byte);
        }

        bytes.push(packet.checksum);

        match self.serial_port.write(&bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn send_keyboard() {}

    pub async fn send_mouse() {}
}
