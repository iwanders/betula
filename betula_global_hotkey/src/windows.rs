use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageA, GetMessageW, PeekMessageA, PostThreadMessageA,
    SetWindowsHookExA, TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG,
    PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::{HotkeyError, Hotkey, HotkeyEvent};

use std::collections::HashMap;
use std::sync::Mutex;

pub type BackendType = InputhookBackend;

pub struct InputhookBackend {
    //manager: GlobalHotKeyManager,
    id_to_hotkey_map: Mutex<std::collections::HashMap<u32, Hotkey>>,
}

impl InputhookBackend {
    pub fn new() -> Result<InputhookBackend, HotkeyError> {
        // let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            // manager,
            id_to_hotkey_map: Default::default(),
        })
    }

    pub fn get_events(&self) -> Result<Vec<HotkeyEvent>, HotkeyError> {
        let mut v = vec![];
        let locked = self.id_to_hotkey_map.lock().unwrap();
        /*
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
        */
        Ok(v)
    }

    pub fn register(&self, key: Hotkey) -> Result<(), HotkeyError> {
        // let hotkey = global_hotkey::hotkey::HotKey::new(Some(key.modifiers), key.key);
        {
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            // locked.insert(hotkey.id(), key.clone());
        }
        // self.manager.register(hotkey)?;
        Ok(())
    }

    pub fn unregister(&self, key: Hotkey) -> Result<(), HotkeyError> {
        // let hotkey = global_hotkey::hotkey::HotKey::new(Some(key.modifiers), key.key);
        {
            // println!("Unregister for {key:?}");
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            // locked.remove(&hotkey.id());
        }
        // self.manager.unregister(hotkey)?;
        Ok(())
    }
}
