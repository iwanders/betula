use betula_global_hotkey::GlobalHotkeyInterface;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut runner = GlobalHotkeyInterface::new()?;

    let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
    runner.register(hotkey)?;
    let hotkey_without_mods = HotKey::new(None, Code::KeyQ);
    runner.register(hotkey_without_mods)?;

    let mut sleep_interval = std::time::Duration::from_millis(100);
    loop {
        std::thread::sleep(sleep_interval);
        for k in [hotkey, hotkey_without_mods] {
            println!(
                "{k:?}: is_pressed: {}, is_toggled: {}",
                runner.is_pressed(k)?,
                runner.is_toggled(k)?
            );
        }
        println!();
    }
    Ok(())
}
