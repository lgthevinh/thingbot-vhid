use slint::{Model, ModelRc, SharedString, VecModel};
use tokio::sync::mpsc::UnboundedSender;

use crate::hid_listener::{ListenerCommand, ListenerStatus};
use crate::hid_serial::HidSerialHandler;

slint::include_modules!();

/// Fills the port list and routes the window's callbacks to the listener as commands.
pub fn bind(window: &MainWindow, commands: UnboundedSender<ListenerCommand>) {
    window.set_ports(scan_ports());

    let weak = window.as_weak();
    window.on_refresh_ports(move || {
        if let Some(window) = weak.upgrade() {
            window.set_ports(scan_ports());
        }
    });

    let weak = window.as_weak();
    let tx = commands.clone();
    window.on_connect(move |index| {
        let port = weak
            .upgrade()
            .and_then(|window| window.get_ports().row_data(index as usize));
        if let Some(port) = port {
            let _ = tx.send(ListenerCommand::Connect(port.into()));
        }
    });

    let tx = commands.clone();
    window.on_disconnect(move || {
        let _ = tx.send(ListenerCommand::Disconnect);
    });

    window.on_toggle_forwarding(move || {
        let _ = commands.send(ListenerCommand::ToggleForwarding);
    });
}

/// Applies a listener status to the window; callable from any thread.
pub fn show_status(weak: &slint::Weak<MainWindow>, status: ListenerStatus) {
    let _ = weak.upgrade_in_event_loop(move |window| {
        window.set_connected(status.connected);
        window.set_forwarding(status.forwarding);
        window.set_devices(to_model(status.devices));
        window.set_error(status.error.into());
    });
}

fn scan_ports() -> ModelRc<SharedString> {
    let ports = HidSerialHandler::scan()
        .unwrap_or_default()
        .into_iter()
        .map(|info| info.port_name)
        .collect();
    to_model(ports)
}

fn to_model(items: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}
