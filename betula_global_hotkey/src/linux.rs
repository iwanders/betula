use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::atomic::AtomicBool;

use crate::{Hotkey as CrateHotkey, HotkeyError, HotkeyEvent};

pub type BackendType = GlobalHotKeyBackend;

pub struct GlobalHotKeyBackend {
    manager: GlobalHotKeyManager,
}

impl GlobalHotKeyBackend {
    pub fn new() -> Result<GlobalHotKeyBackend, HotkeyError> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self { manager })
    }

    pub fn get_events(&self) -> Result<Vec<HotkeyEvent>, HotkeyError> {
        let mut v = vec![];
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            println!("{:?}", event);
        }
        Ok(v)
    }

    pub fn register(&self, key: CrateHotkey) -> Result<(), HotkeyError> {
        let hotkey = global_hotkey::hotkey::HotKey::new(Some(key.modifiers), key.key);
        self.manager.register(hotkey)?;
        Ok(())
    }
}

pub struct GlobalHotkeyRunner {
    thread: Option<std::thread::JoinHandle<()>>,
    running: std::sync::Arc<AtomicBool>,
}

impl Drop for GlobalHotkeyRunner {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let t = self.thread.take();
        t.unwrap().join().expect("join should succeed");
    }
}

/*


*/
