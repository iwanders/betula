use betula_core::BetulaError;
use serde::{Deserialize, Serialize};

pub mod nodes;

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

// From the docs
// On Windows a win32 event loop must be running on the thread. It doesn’t need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.

use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

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

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct GlobalHotkeyInterface {
    manager: Arc<Mutex<GlobalHotKeyManager>>,

    // dead code allowed, it contains the execution thread.
    #[allow(dead_code)]
    runner: Arc<GlobalHotkeyRunner>,
}

impl GlobalHotkeyInterface {
    pub fn new() -> Result<GlobalHotkeyInterface, BetulaError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let t_running = running.clone();

        // Channel to exfiltrate the pointer from the thread servicing the manager.
        use std::sync::mpsc::channel;
        let (sender, receiver) = channel::<Option<Arc<Mutex<GlobalHotKeyManager>>>>();
        let thread = Some(std::thread::spawn(move || {
            // Spawn the manager in the thread that will service it.
            // This will likely also need an event loop for windows.
            let manager;
            if let Ok(manager_res) = GlobalHotKeyManager::new() {
                manager = Arc::new(Mutex::new(manager_res));
            } else {
                return;
            }

            if sender.send(Some(Arc::clone(&manager))).is_err() {
                return;
            }

            {
                let mut locked = manager.lock().unwrap();
                let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
                let hotkey_without_mods = HotKey::new(None, Code::KeyQ);
                locked.register(hotkey).unwrap();
                locked.register(hotkey_without_mods).unwrap();
            }

            while t_running.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(event) =
                    GlobalHotKeyEvent::receiver().recv_timeout(std::time::Duration::from_millis(1))
                {
                    println!("{:?}", event);
                }
            }
        }));

        let manager = receiver.recv_timeout(std::time::Duration::from_millis(1000))?;
        let manager = if let Some(manager) = manager {
            manager
        } else {
            return Err(format!("failed to create hotkey manager").into());
        };

        let runner = Arc::new(GlobalHotkeyRunner { thread, running });

        Ok(GlobalHotkeyInterface { runner, manager })
    }
}

impl std::fmt::Debug for GlobalHotkeyInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "GlobalHotkeyRunner<{:?}>", Arc::as_ptr(&self.runner))
    }
}
impl std::cmp::PartialEq for GlobalHotkeyInterface {
    fn eq(&self, other: &GlobalHotkeyInterface) -> bool {
        Arc::as_ptr(&self.runner) == Arc::as_ptr(&other.runner)
    }
}

/// Register global_hotkey nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {}
