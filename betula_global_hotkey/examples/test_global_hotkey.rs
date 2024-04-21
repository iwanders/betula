use betula_global_hotkey::GlobalHotkeyInterface;

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runner = GlobalHotkeyInterface::new()?;

    let mut sleep_interval = std::time::Duration::from_millis(10);
    loop {
        std::thread::sleep(sleep_interval);
    }
    Ok(())
}
