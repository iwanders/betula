use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::{Hotkey as CrateHotkey, HotkeyError, HotkeyEvent, KeyState};

pub type BackendType = GlobalHotKeyBackend;

use std::sync::Mutex;

pub struct GlobalHotKeyBackend {
    manager: GlobalHotKeyManager,
    id_to_hotkey_map: Mutex<std::collections::HashMap<u32, CrateHotkey>>,
}

impl GlobalHotKeyBackend {
    pub fn new() -> Result<GlobalHotKeyBackend, HotkeyError> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            manager,
            id_to_hotkey_map: Default::default(),
        })
    }

    pub fn get_events(&self) -> Result<Vec<HotkeyEvent>, HotkeyError> {
        let mut v = vec![];
        let locked = self.id_to_hotkey_map.lock().unwrap();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            // Translate the event
            if let Some(hk) = locked.get(&event.id) {
                let state = if event.state == HotKeyState::Pressed {
                    KeyState::Down
                } else {
                    KeyState::Up
                };
                let event = HotkeyEvent {
                    state,
                    hotkey: hk.clone(),
                };
                v.push(event);
            }
        }
        Ok(v)
    }

    pub fn register(&self, key: CrateHotkey) -> Result<(), HotkeyError> {
        let hotkey = global_hotkey::hotkey::HotKey::new(Some(key.modifiers), key.key);
        {
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            locked.insert(hotkey.id(), key.clone());
        }
        self.manager.register(hotkey)?;
        Ok(())
    }
    pub fn unregister(&self, key: CrateHotkey) -> Result<(), HotkeyError> {
        let hotkey = global_hotkey::hotkey::HotKey::new(Some(key.modifiers), key.key);
        {
            println!("Unregister for {key:?}");
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            locked.remove(&hotkey.id());
        }
        self.manager.unregister(hotkey)?;
        Ok(())
    }
}
