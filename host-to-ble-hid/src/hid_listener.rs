use std::collections::BTreeSet;
use std::future::poll_fn;
use std::io;
use std::task::Poll;

use evdev::{
    EventStream, EventSummary, InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode,
};

use crate::hid_serial::HidSerialHandler;
use crate::hid_serial::hid_serial_keyboard::HidSerialKeyboardReport;
use crate::hid_serial::hid_serial_mouse::{self, HidSerialMouseReport};

/// Reads every keyboard and mouse through evdev, grabs them and forwards their input as serial HID reports.
/// Alt+Shift+F12 toggles forwarding.
pub struct HidListener {
    streams: Vec<EventStream>,
    serial: HidSerialHandler,
    pressed: BTreeSet<KeyCode>,
    forwarding: bool,
    pending_grab: bool,
    motion: (i32, i32, i32),
    last_keyboard: HidSerialKeyboardReport,
    last_buttons: u8,
}

impl HidListener {
    pub fn open(serial: HidSerialHandler) -> io::Result<Self> {
        let mut streams = Vec::new();
        let mut pressed = BTreeSet::new();

        for (path, device) in evdev::enumerate() {
            let keys = device.supported_keys();
            let is_keyboard = keys.is_some_and(|k| k.contains(KeyCode::KEY_A));
            let is_mouse = keys.is_some_and(|k| k.contains(KeyCode::BTN_LEFT))
                && device
                    .supported_relative_axes()
                    .is_some_and(|r| r.contains(RelativeAxisCode::REL_X));
            if !is_keyboard && !is_mouse {
                continue;
            }

            // Keys held at startup (e.g. Enter that launched us) must be released before grabbing
            pressed.extend(device.get_key_state()?.iter());
            tracing::info!(
                "using {} ({})",
                path.display(),
                device.name().unwrap_or("?")
            );
            streams.push(device.into_event_stream()?);
        }

        if streams.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no keyboard or mouse found",
            ));
        }

        Ok(Self {
            streams,
            serial,
            pressed,
            forwarding: false,
            pending_grab: true,
            motion: (0, 0, 0),
            last_keyboard: HidSerialKeyboardReport::default(),
            last_buttons: 0,
        })
    }

    pub async fn run(&mut self) -> io::Result<()> {
        self.try_grab()?;
        loop {
            let event = match self.next_event().await {
                Ok(event) => event,
                Err(e) => {
                    // Device gone: don't leave keys held on the target
                    let _ = self.release().await;
                    return Err(e);
                }
            };
            self.handle(event).await?;
        }
    }

    /// Waits for the next event from any device.
    async fn next_event(&mut self) -> io::Result<InputEvent> {
        poll_fn(|cx| {
            for stream in &mut self.streams {
                if let Poll::Ready(result) = stream.poll_event(cx) {
                    return Poll::Ready(result);
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
        if self.forwarding {
            self.forwarding = false;
            self.release().await?;
            for stream in &mut self.streams {
                stream.device_mut().ungrab()?;
            }
            tracing::info!("forwarding off");
        } else {
            self.pending_grab = true;
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
        tracing::info!("forwarding on");
        Ok(())
    }

    /// Sends the reports that changed since the last sync.
    async fn flush(&mut self) -> io::Result<()> {
        let keyboard = HidSerialKeyboardReport::from_pressed(&self.pressed);
        if keyboard != self.last_keyboard {
            self.serial.send_keyboard(&keyboard).await?;
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
                self.serial.send_mouse(&report).await?;
            }
            self.last_buttons = buttons;
        }
        Ok(())
    }

    /// Sends empty reports so nothing stays pressed on the target.
    async fn release(&mut self) -> io::Result<()> {
        self.last_keyboard = HidSerialKeyboardReport::default();
        self.last_buttons = 0;
        self.serial.send_keyboard(&self.last_keyboard).await?;
        self.serial
            .send_mouse(&HidSerialMouseReport::default())
            .await
    }
}
