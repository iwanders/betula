use betula_global_hotkey::{Code, GlobalHotkeyInterface, Hotkey, Modifiers};

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut runner = GlobalHotkeyInterface::new()?;

    let mut sleep_interval = std::time::Duration::from_millis(100);

    let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyD);
    let token = runner.register(hotkey)?;
    {
        let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyA);
        let token = runner.register(hotkey)?;
    }

    let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyD);
    {
        let token2 = runner.register(hotkey)?;
    }

    loop {
        std::thread::sleep(sleep_interval);
        println!("token: {}, {}", token.is_pressed(), token.is_toggled());
        println!();
    }

    Ok(())
}
