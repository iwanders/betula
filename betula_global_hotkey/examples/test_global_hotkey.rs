use betula_global_hotkey::{Code, GlobalHotkeyInterface, Hotkey, Modifiers};

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut runner = GlobalHotkeyInterface::new()?;

    let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyD);
    runner.register(hotkey)?;

    let mut sleep_interval = std::time::Duration::from_millis(100);
    loop {
        std::thread::sleep(sleep_interval);

        println!();
    }
    Ok(())
}
