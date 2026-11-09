use host_to_ble_hid::hid_listener::HidListener;
use host_to_ble_hid::ui::{self, MainWindow};
use slint::ComponentHandle;
use tokio::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let (commands, receiver) = mpsc::unbounded_channel();
    ui::bind(&window, commands);

    // Slint needs the main thread, so the listener gets its own runtime on a background thread.
    // It is opened inside the runtime because evdev streams register with tokio's reactor.
    let weak = window.as_weak();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let listener_thread = std::thread::spawn(move || {
        runtime.block_on(async {
            let on_status = Box::new(move |status| ui::show_status(&weak, status));
            HidListener::open(on_status).run(receiver).await
        })
    });

    window.run()?;

    // Dropping the window drops the command sender, which makes the listener release and exit
    drop(window);
    let _ = listener_thread.join();
    Ok(())
}
