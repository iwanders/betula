use betula_hotkey::{Code, Hotkey, HotkeyInterface, Modifiers};

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runner = HotkeyInterface::new()?;

    let sleep_interval = std::time::Duration::from_millis(100);

    let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyD);
    let token = runner.register(hotkey)?;
    {
        let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyA);
        let _token = runner.register(hotkey)?;
    }

    let hotkey = Hotkey::new(Some(Modifiers::SHIFT), Code::KeyD);
    {
        let _token2 = runner.register(hotkey)?;
    }

    loop {
        std::thread::sleep(sleep_interval);
        println!(
            "Key {:?}: {}, {}",
            token.hotkey(),
            token.is_pressed(),
            token.depress_count()
        );
        println!();
    }
}
