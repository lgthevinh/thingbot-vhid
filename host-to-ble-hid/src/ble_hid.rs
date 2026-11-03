use btleplug::{
    api::{Central, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral},
};
use futures::lock::Mutex;

pub struct BleHidDeviceScan {
    name: String,
    mac: String,
    rssi: i16,
    peripheral: Peripheral
}

pub struct BleHidHostHandler {
    adapter: Adapter,
    connected: Mutex<Vec<Peripheral>>,
}

impl BleHidHostHandler {
    pub async fn new() -> Result<Self, btleplug::Error> {
        let manager = Manager::new().await.unwrap();
        let adapter = manager
            .adapters()
            .await
            .unwrap()
            .into_iter()
            .nth(0)
            .unwrap();

        Ok(Self {
            adapter,
            connected: Mutex::new(Vec::new()),
        })
    }

    pub async fn scan(&mut self) -> Result<Vec<BleHidDeviceScan>, btleplug::Error> {
        // todo: filter out only thingbot ble hid
        let filter = ScanFilter::default();

        let session = &self.adapter;
        session.start_scan(filter).await.unwrap();

        let mut scanned_devices = Vec::new();
        for peripheral in session.peripherals().await.unwrap() {
            let props = peripheral.properties().await.unwrap().unwrap();
            match props.local_name {
                Some(name) => scanned_devices.push(BleHidDeviceScan {
                    name: name,
                    mac: props.address.to_string(),
                    rssi: props.rssi.unwrap(),
                    peripheral: peripheral
                }),
                None => {
                    continue;
                }
            }
        }

        Ok(scanned_devices)
    }
}
