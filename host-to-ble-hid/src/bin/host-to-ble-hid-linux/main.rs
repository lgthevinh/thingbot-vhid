use host_to_ble_hid::hid_listener::HidListener;
use host_to_ble_hid::hid_serial::{HidSerialDeviceInfo, HidSerialHandler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port_name = std::env::args()
        .nth(1)
        .ok_or("usage: host-to-ble-hid-linux <serial-port>")?;
    let serial = HidSerialHandler::connect(&HidSerialDeviceInfo { port_name })?;
    let mut listener = HidListener::open(serial)?;
    listener.run().await?;
    Ok(())
}
