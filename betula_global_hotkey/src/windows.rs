use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageA, GetMessageW, PeekMessageA, PostThreadMessageA,
    SetWindowsHookExA, TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG,
    PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::{HotkeyError, Hotkey, HotkeyEvent, KeyState};

use std::collections::HashMap;
use std::sync::{Arc,Mutex, atomic::{AtomicU32, AtomicBool}};
use std::cell::RefCell;

pub type BackendType = InputhookBackend;




/// Single global hook handler function, it uses the thread_local in InputHook to dispatch appropriately.
///
/// code: A code the hook procedure uses to determine how to process the message. If nCode is less than zero, the hook procedure must pass the message to the CallNextHookEx function without further processing and should return the value returned by CallNextHookEx. This parameter can be one of the following values.
/// Value	Meaning
/// HC_ACTION 0
/// The wParam and lParam parameters contain information about a keyboard message.
/// wparam: The identifier of the keyboard message. This parameter can be one of the following messages: WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, or WM_SYSKEYUP.
/// lparam: Type: LPARAM
/// A pointer to a KBDLLHOOKSTRUCT structure.
unsafe extern "system" fn hook_handler(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(HHOOK(0), code, wparam, lparam);
    }

    let z = std::mem::transmute::<_, *const KBDLLHOOKSTRUCT>(lparam);

    let action = match wparam.0 as u32 {
        WM_KEYDOWN => KeyState::Down,
        WM_KEYUP => KeyState::Up,
        WM_SYSKEYDOWN => KeyState::Down,
        WM_SYSKEYUP => KeyState::Up,
        _ => {
            panic!("unsupported key action {}", wparam.0);
        }
    };
    println!("code: {}", (*z).vkCode);
    /*
    let input = KeyInput {
        action,
        vk_code: (*z).vkCode as u8,
    };
    let time = EventTime((*z).time);
    InputhookBackend::MAP.with(|z| {
        let mut l = z.borrow_mut();
        if let Some(f) = l.get_mut(&input) {
            f(input, time);
        }
    });
    */

    return CallNextHookEx(HHOOK(0), code, wparam, lparam);
}

struct VirtualKeyEvent{
}

type HotkeyMap = Arc<Mutex<std::collections::HashMap<u32, Hotkey>>>;
use std::sync::mpsc::{Sender, Receiver, channel};
pub struct InputhookBackend {
    //manager: GlobalHotKeyManager,
    id_to_hotkey_map: HotkeyMap,

    receiver: Receiver<VirtualKeyEvent>,
    thread: Option<std::thread::JoinHandle<()>>,
    thread_id: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl InputhookBackend {
    thread_local! {
        /// Thread local map to store the callbacks used for this hook handler.
        static LOCAL_SENDER: RefCell<Option<(Sender<VirtualKeyEvent>, HotkeyMap)>> = Default::default();
    }

    pub fn new() -> Result<InputhookBackend, HotkeyError> {
        let thread_id = std::sync::Arc::new(AtomicU32::new(0));
        let running = std::sync::Arc::new(AtomicBool::new(true));

        let (sender, receiver) = channel::<VirtualKeyEvent>();

        let id_to_hotkey_map : HotkeyMap = Default::default();

        let tid = thread_id.clone();
        let t_running = running.clone();
        let t_id_to_hotkey_map = Arc::clone(&id_to_hotkey_map);
        let thread = Some(std::thread::spawn(move || {
            unsafe {
                // Assign the sender.
                InputhookBackend::LOCAL_SENDER.with(|z| {
                    let mut l = z.borrow_mut();
                    *l = Some((sender, t_id_to_hotkey_map));
                });


                // Store the thread id such that we can later exit this thread by sending a message.
                let current_id = GetCurrentThreadId();
                tid.store(current_id, std::sync::atomic::Ordering::Relaxed);

                // Set the hook.
                let hh = SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_handler), HINSTANCE(0), 0)
                    .expect("hook did not succeed");

                // https://stackoverflow.com/a/65571485
                // This hook is called in the context of the thread that installed it. The call is made by sending a message to the thread that installed the hook. Therefore, the thread that installed the hook must have a message loop, which we place here.
                // I think other systems could send messages, to lets wrap it into a loop with a mutex to quit if we
                // really want to.
                while t_running.load(std::sync::atomic::Ordering::Relaxed) {
                    let mut message: MSG = std::mem::zeroed();
                    let _ = GetMessageW(&mut message, HWND(0), 0, 0);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                // Unhook the things.
                UnhookWindowsHookEx(hh).expect("unhook did not succeed");
            }
        }));

        Ok(Self {
            thread,
            running,
            thread_id,
            receiver,
            id_to_hotkey_map,
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


impl Drop for InputhookBackend {
    fn drop(&mut self) {
        // Set the boolean to stop running.
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);

        unsafe {
            // Send a message to the thread to drop out of the GetMessage loop.
            let tid = self.thread_id.load(std::sync::atomic::Ordering::Relaxed);
            PostThreadMessageA(tid, 0, WPARAM(0), LPARAM(0)).expect("other thread must be running");
        }

        // Finally, join the thread.
        self.thread.take().unwrap().join().expect("join should succeed");
    }
}

