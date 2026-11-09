use std::collections::BTreeSet;
use std::future::poll_fn;
use std::io;
use std::task::Poll;

use evdev::{
    EventStream, EventSummary, InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::hid_serial::hid_serial_keyboard::HidSerialKeyboardReport;
use crate::hid_serial::hid_serial_mouse::{self, HidSerialMouseReport};
use crate::hid_serial::{HidSerialDeviceInfo, HidSerialHandler};

pub enum ListenerCommand {
    Connect(String),
    Disconnect,
    ToggleForwarding,
}

#[derive(Clone, PartialEq, Default)]
pub struct ListenerStatus {
    pub connected: bool,
    pub forwarding: bool,
    pub devices: Vec<String>,
    pub error: String,
}

/// Reads every keyboard and mouse through evdev and, while forwarding, grabs them and sends their input as serial HID reports.
/// Alt+Shift+F12 toggles forwarding.
pub struct HidListener {
    streams: Vec<EventStream>,
    names: Vec<String>,
    serial: Option<HidSerialHandler>,
    pressed: BTreeSet<KeyCode>,
    forwarding: bool,
    pending_grab: bool,
    motion: (i32, i32, i32),
    last_keyboard: HidSerialKeyboardReport,
    last_buttons: u8,
    error: String,
    on_status: Box<dyn Fn(ListenerStatus) + Send>,
    last_status: Option<ListenerStatus>,
}

impl HidListener {
    /// Opens all keyboards and mice; must be called inside a tokio runtime.
    pub fn open(on_status: Box<dyn Fn(ListenerStatus) + Send>) -> Self {
        let mut streams = Vec::new();
        let mut names = Vec::new();
        let mut pressed = BTreeSet::new();

        for (_, device) in evdev::enumerate() {
            let keys = device.supported_keys();
            let is_keyboard = keys.is_some_and(|k| k.contains(KeyCode::KEY_A));
            let is_mouse = keys.is_some_and(|k| k.contains(KeyCode::BTN_LEFT))
                && device
                    .supported_relative_axes()
                    .is_some_and(|r| r.contains(RelativeAxisCode::REL_X));
            if !is_keyboard && !is_mouse {
                continue;
            }

            // Keys already held must be released before the first grab
            let Ok(state) = device.get_key_state() else {
                continue;
            };
            let name = device.name().unwrap_or("?").to_string();
            let Ok(stream) = device.into_event_stream() else {
                continue;
            };
            pressed.extend(state.iter());
            names.push(name);
            streams.push(stream);
        }

        let error = if streams.is_empty() {
            "no readable keyboard or mouse; is the user in the input group?".to_string()
        } else {
            String::new()
        };

        Self {
            streams,
            names,
            serial: None,
            pressed,
            forwarding: false,
            pending_grab: false,
            motion: (0, 0, 0),
            last_keyboard: HidSerialKeyboardReport::default(),
            last_buttons: 0,
            error,
            on_status,
            last_status: None,
        }
    }

    /// Runs until the command sender is dropped.
    pub async fn run(mut self, mut commands: UnboundedReceiver<ListenerCommand>) {
        loop {
            self.publish();
            tokio::select! {
                command = commands.recv() => {
                    let Some(command) = command else {
                        self.stop_forwarding().await;
                        return;
                    };
                    if let Err(e) = self.handle_command(command).await {
                        self.fail(e).await;
                    }
                }
                (index, result) = self.next_event() => match result {
                    Ok(event) => {
                        if let Err(e) = self.handle(event).await {
                            self.fail(e).await;
                        }
                    }
                    Err(e) => self.remove_device(index, e).await,
                },
            }
        }
    }

    /// Sends the current status if it changed since the last call.
    fn publish(&mut self) {
        let status = ListenerStatus {
            connected: self.serial.is_some(),
            forwarding: self.forwarding,
            devices: self.names.clone(),
            error: self.error.clone(),
        };
        if self.last_status.as_ref() != Some(&status) {
            (self.on_status)(status.clone());
            self.last_status = Some(status);
        }
    }

    async fn handle_command(&mut self, command: ListenerCommand) -> io::Result<()> {
        match command {
            ListenerCommand::Connect(port_name) => {
                self.serial = Some(HidSerialHandler::connect(&HidSerialDeviceInfo {
                    port_name,
                })?);
                self.error.clear();
            }
            ListenerCommand::Disconnect => {
                self.stop_forwarding().await;
                self.serial = None;
            }
            ListenerCommand::ToggleForwarding => self.toggle().await?,
        }
        Ok(())
    }

    /// Waits for the next event from any device, returning the device's index with it.
    async fn next_event(&mut self) -> (usize, io::Result<InputEvent>) {
        poll_fn(|cx| {
            for (index, stream) in self.streams.iter_mut().enumerate() {
                if let Poll::Ready(result) = stream.poll_event(cx) {
                    return Poll::Ready((index, result));
                }
            }
            Poll::Pending
        })
        .await
    }

    async fn handle(&mut self, event: InputEvent) -> io::Result<()> {
        match event.destructure() {
            EventSummary::Key(_, key, value) => self.handle_key(key, value).await,
            EventSummary::RelativeAxis(_, axis, value) if self.forwarding => {
                match axis {
                    RelativeAxisCode::REL_X => self.motion.0 += value,
                    RelativeAxisCode::REL_Y => self.motion.1 += value,
                    RelativeAxisCode::REL_WHEEL => self.motion.2 += value,
                    _ => {}
                }
                Ok(())
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _)
                if self.forwarding =>
            {
                self.flush().await
            }
            _ => Ok(()),
        }
    }

    async fn handle_key(&mut self, key: KeyCode, value: i32) -> io::Result<()> {
        match value {
            1 => {
                if key == KeyCode::KEY_F12
                    && self.is_held_any(KeyCode::KEY_LEFTALT, KeyCode::KEY_RIGHTALT)
                    && self.is_held_any(KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_RIGHTSHIFT)
                {
                    self.toggle().await?;
                }
                self.pressed.insert(key);
            }
            0 => {
                self.pressed.remove(&key);
            }
            // Auto-repeat: the target generates its own
            _ => return Ok(()),
        }
        self.try_grab()
    }

    fn is_held_any(&self, a: KeyCode, b: KeyCode) -> bool {
        self.pressed.contains(&a) || self.pressed.contains(&b)
    }

    async fn toggle(&mut self) -> io::Result<()> {
        if self.forwarding || self.pending_grab {
            self.stop_forwarding().await;
        } else if self.serial.is_some() {
            self.pending_grab = true;
            self.try_grab()?;
        }
        Ok(())
    }

    /// Grabs all devices once a grab is pending and no key is held.
    /// Grabbing with a key down would swallow its release and leave it stuck on the host.
    fn try_grab(&mut self) -> io::Result<()> {
        if !self.pending_grab || !self.pressed.is_empty() {
            return Ok(());
        }
        for stream in &mut self.streams {
            stream.device_mut().grab()?;
        }
        self.pending_grab = false;
        self.forwarding = true;
        self.motion = (0, 0, 0);
        Ok(())
    }

    /// Releases everything on the target and ungrabs; best effort since it also runs after failures.
    async fn stop_forwarding(&mut self) {
        self.pending_grab = false;
        if self.forwarding {
            self.forwarding = false;
            let _ = self.release().await;
        }
        for stream in &mut self.streams {
            let _ = stream.device_mut().ungrab();
        }
    }

    /// Stops forwarding and drops the serial port after an error.
    async fn fail(&mut self, error: io::Error) {
        self.error = error.to_string();
        self.stop_forwarding().await;
        self.serial = None;
    }

    /// Drops a device whose stream failed (usually unplugged).
    async fn remove_device(&mut self, index: usize, error: io::Error) {
        self.stop_forwarding().await;
        self.streams.remove(index);
        let name = self.names.remove(index);
        self.error = format!("{name}: {error}");

        // Its held keys can never be released now, so rebuild the set from the remaining devices
        self.pressed = self
            .streams
            .iter()
            .filter_map(|stream| stream.device().get_key_state().ok())
            .flat_map(|keys| keys.iter().collect::<Vec<_>>())
            .collect();
    }

    /// Sends the reports that changed since the last sync.
    async fn flush(&mut self) -> io::Result<()> {
        let Some(serial) = &mut self.serial else {
            return Ok(());
        };

        let keyboard = HidSerialKeyboardReport::from_pressed(&self.pressed);
        if keyboard != self.last_keyboard {
            serial.send_keyboard(&keyboard).await?;
            self.last_keyboard = keyboard;
        }

        let buttons = self
            .pressed
            .iter()
            .filter_map(|&key| hid_serial_mouse::button_bit(key))
            .fold(0, |acc, bit| acc | bit);
        let (x, y, wheel) = std::mem::take(&mut self.motion);
        if buttons != self.last_buttons || (x, y, wheel) != (0, 0, 0) {
            for report in hid_serial_mouse::split_motion(buttons, x, y, wheel) {
                serial.send_mouse(&report).await?;
            }
            self.last_buttons = buttons;
        }
        Ok(())
    }

    /// Sends empty reports so nothing stays pressed on the target.
    async fn release(&mut self) -> io::Result<()> {
        self.last_keyboard = HidSerialKeyboardReport::default();
        self.last_buttons = 0;
        let Some(serial) = &mut self.serial else {
            return Ok(());
        };
        serial.send_keyboard(&self.last_keyboard).await?;
        serial.send_mouse(&HidSerialMouseReport::default()).await
    }
}
