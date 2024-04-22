use betula_core::BetulaError;

use std::collections::HashMap;

pub mod nodes;

use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW,PeekMessageA , PostThreadMessageA, SetWindowsHookExA, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,PM_REMOVE,
                TranslateMessage,
                DispatchMessageW,
                GetMessageA,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

// From the docs
// On Windows a win32 event loop must be running on the thread. It doesn’t need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
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

#[derive(Debug, Default)]
struct State {
    /// Whether the key is currently depressed.
    is_pressed: AtomicBool,
    /// Boolean that's toggled when the key is depressed.
    is_toggled: AtomicBool,
}
type StatePtr = Arc<State>;
type StateMap = HashMap<HotKeyId, StatePtr>;
type HotKeyId = u32;

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct GlobalHotkeyInterface {
    /// Pointer to the actual manager used by the runner.
    manager: Arc<Mutex<GlobalHotKeyManager>>,

    /// Actual map of states that gets updated.
    state: StateMap,

    /// Sender to provide the runner thread with the map to be updated.
    state_sender: Sender<StateMap>,

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
        let (state_sender, state_receiver) = channel::<StateMap>();
        let thread = Some(std::thread::spawn(move || {
            let mut our_state_map = StateMap::default();

            // from https://stackoverflow.com/a/51943720
            // The queue only gets created when we look at the queue from a thread.
            unsafe {
                let mut msg : MSG = Default::default();
                PeekMessageA(&mut msg, HWND(0), 0, 0, PM_REMOVE);
                let current_id = GetCurrentThreadId();
                PostThreadMessageA(current_id, 0, WPARAM(0), LPARAM(0)).expect("other thread must be running");
                GetMessageA(&mut msg, HWND(0), 0, 0);
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            //
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

            while t_running.load(Relaxed) {
                if let Ok(event) =
                    GlobalHotKeyEvent::receiver().recv_timeout(std::time::Duration::from_millis(1))
                {
                    if let Some(v) = our_state_map.get(&event.id()) {
                        v.is_pressed
                            .store(event.state == global_hotkey::HotKeyState::Pressed, Relaxed);
                        if event.state == global_hotkey::HotKeyState::Pressed {
                            v.is_toggled.fetch_xor(true, Relaxed);
                        }
                    }
                    println!("{:?}", our_state_map);
                }
                if let Ok(v) = state_receiver.try_recv() {
                    our_state_map = v;
                }

                // process windows event loop.
                /*
                {

                    MSG msg = { };
                    while (GetMessage(&msg, NULL, 0, 0) > 0)
                    {
                        TranslateMessage(&msg);
                        DispatchMessage(&msg);
                    }
                }*/

            }
        }));

        let manager = receiver.recv_timeout(std::time::Duration::from_millis(1000))?;
        let manager = if let Some(manager) = manager {
            manager
        } else {
            return Err(format!("failed to create hotkey manager").into());
        };

        let runner = Arc::new(GlobalHotkeyRunner { thread, running });
        let state = Default::default();
        Ok(GlobalHotkeyInterface {
            runner,
            manager,
            state,
            state_sender,
        })
    }

    pub fn register(&mut self, hotkey: HotKey) -> Result<(), BetulaError> {
        let locked = self.manager.lock().unwrap();
        locked.register(hotkey)?;
        self.state
            .entry(hotkey.id())
            .or_insert_with(|| Arc::new(Default::default()));
        self.state_sender.send(self.state.clone())?;
        Ok(())
    }
    pub fn unregister(&mut self, hotkey: HotKey) -> Result<(), BetulaError> {
        let locked = self.manager.lock().unwrap();
        locked.unregister(hotkey)?;
        self.state.remove(&hotkey.id());
        self.state_sender.send(self.state.clone())?;
        Ok(())
    }

    pub fn is_pressed(&self, hotkey: HotKey) -> Result<bool, BetulaError> {
        if let Some(v) = self.state.get(&hotkey.id()) {
            Ok(v.is_pressed.load(Relaxed))
        } else {
            Err(format!("hotkey {hotkey:?} not registered").into())
        }
    }

    pub fn is_toggled(&self, hotkey: HotKey) -> Result<bool, BetulaError> {
        if let Some(v) = self.state.get(&hotkey.id()) {
            Ok(v.is_toggled.load(Relaxed))
        } else {
            Err(format!("hotkey {hotkey:?} not registered").into())
        }
    }
}

impl std::fmt::Debug for GlobalHotkeyInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "GlobalHotkeyRunner<{:?}>", Arc::as_ptr(&self.runner))
    }
}

/// Register global_hotkey nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {}
