use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW,  PostThreadMessageA,
    SetWindowsHookExA, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT};
use crate::{HotkeyError, Hotkey, HotkeyEvent, KeyState};

use std::sync::{Arc,Mutex, atomic::{AtomicU32, AtomicBool}};
use std::cell::RefCell;

pub type BackendType = InputhookBackend;


// https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes

// https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input


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
    if code < 0 || lparam == LPARAM(0) {
        return CallNextHookEx(HHOOK(0), code, wparam, lparam);
    }

    let z = std::mem::transmute::<_, *const KBDLLHOOKSTRUCT>(lparam);
    let state = match wparam.0 as u32 {
        WM_KEYDOWN => KeyState::Down,
        WM_KEYUP => KeyState::Up,
        WM_SYSKEYDOWN => KeyState::Down,
        WM_SYSKEYUP => KeyState::Up,
        _ => {
            panic!("unsupported key action {}", wparam.0);
        }
    };
    // f1
    // code: KBDLLHOOKSTRUCT { vkCode: 112, scanCode: 59, flags: KBDLLHOOKSTRUCT_FLAGS(0), time: 4147093, dwExtraInfo: 0 }

    let event = &*z;
    // println!("event: {event:?}");

    let virtual_key = event.vkCode;

    // Get the modifiers with https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getkeystate
    let control_down = is_key_down(VK_CONTROL.into());
    let alt_down = is_key_down(VK_MENU.into());
    let shift_down = is_key_down(VK_SHIFT.into());
    let meta_down = is_key_down(VK_RWIN.into()) || is_key_down(VK_LWIN.into());


    let winkey = WindowsHotkey{
        vk: virtual_key as u16,
        control_down,
        alt_down,
        shift_down,
        meta_down,
    };

    

    InputhookBackend::LOCAL_SENDER.with(|z| {
        let l = z.borrow_mut();
        if let Some((sender, map)) = l.as_ref() {
            let locked_map = map.lock().unwrap();
            if let Some(hotkey) = locked_map.get(&winkey) {
                let _ = sender.send(HotkeyEvent{
                    state,
                    hotkey: *hotkey
                });
            }
        }
    });

    return CallNextHookEx(HHOOK(0), code, wparam, lparam);
}

#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
struct VirtualKey {
    code: i32,
}

impl From<VIRTUAL_KEY> for VirtualKey {
    fn from(v: VIRTUAL_KEY) -> Self {
        Self {
            code: v.0 as i32
        }
    }
}



unsafe fn is_key_down(vk: VirtualKey) -> bool {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getkeystate#return-value
    // higher order bit is key depressed
    u16::from_ne_bytes(GetKeyState(vk.code).to_ne_bytes()) & 0x8000u16 != 0
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
struct WindowsHotkey{
    pub vk: u16,
    pub alt_down: bool,
    pub meta_down: bool,
    pub control_down: bool,
    pub shift_down: bool,
}


impl TryFrom<Hotkey> for WindowsHotkey {
    type Error = String;
    fn try_from(k: Hotkey) -> Result<Self, Self::Error> {
        let vk = conversion::key_to_vk(&k.key).ok_or(format!("no vk for {}", k.key))?.0;
        let alt_down = k.modifiers.contains(keyboard_types::Modifiers::ALT);
        let meta_down = k.modifiers.contains(keyboard_types::Modifiers::META);
        let control_down = k.modifiers.contains(keyboard_types::Modifiers::CONTROL);
        let shift_down = k.modifiers.contains(keyboard_types::Modifiers::SHIFT);
        Ok(Self {
            vk,
            alt_down,
            meta_down,
            control_down,
            shift_down,
        })
    }
}

type HotkeyMap = Arc<Mutex<std::collections::HashMap<WindowsHotkey, Hotkey>>>;
use std::sync::mpsc::{Sender, Receiver, channel};
pub struct InputhookBackend {
    //manager: GlobalHotKeyManager,
    id_to_hotkey_map: HotkeyMap,

    receiver: Receiver<HotkeyEvent>,
    thread: Option<std::thread::JoinHandle<()>>,
    thread_id: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl InputhookBackend {
    thread_local! {
        /// Thread local map to store the callbacks used for this hook handler.
        static LOCAL_SENDER: RefCell<Option<(Sender<HotkeyEvent>, HotkeyMap)>> = Default::default();
    }

    pub fn new() -> Result<InputhookBackend, HotkeyError> {
        let thread_id = std::sync::Arc::new(AtomicU32::new(0));
        let running = std::sync::Arc::new(AtomicBool::new(true));

        let (sender, receiver) = channel::<HotkeyEvent>();

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
        while let Ok(event) = self.receiver.try_recv() {
            v.push(event);
        }
        Ok(v)
    }

    pub fn register(&self, key: Hotkey) -> Result<(), HotkeyError> {
        {
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            locked.insert(key.try_into()?, key);
        }
        Ok(())
    }

    pub fn unregister(&self, key: Hotkey) -> Result<(), HotkeyError> {
        {
            let mut locked = self.id_to_hotkey_map.lock().unwrap();
            locked.remove(&key.try_into()?);
        }
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


// conversion function is taken from https://github.com/tauri-apps/global-hotkey/blob/cd9051d725fe830f407cf75603c2e90443158ed6/src/platform_impl/windows/mod.rs
mod conversion {
    use crate::Code;
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    // used to build accelerators table from Key
    pub fn key_to_vk(key: &Code) -> Option<VIRTUAL_KEY> {
        Some(match key {
            Code::KeyA => VK_A,
            Code::KeyB => VK_B,
            Code::KeyC => VK_C,
            Code::KeyD => VK_D,
            Code::KeyE => VK_E,
            Code::KeyF => VK_F,
            Code::KeyG => VK_G,
            Code::KeyH => VK_H,
            Code::KeyI => VK_I,
            Code::KeyJ => VK_J,
            Code::KeyK => VK_K,
            Code::KeyL => VK_L,
            Code::KeyM => VK_M,
            Code::KeyN => VK_N,
            Code::KeyO => VK_O,
            Code::KeyP => VK_P,
            Code::KeyQ => VK_Q,
            Code::KeyR => VK_R,
            Code::KeyS => VK_S,
            Code::KeyT => VK_T,
            Code::KeyU => VK_U,
            Code::KeyV => VK_V,
            Code::KeyW => VK_W,
            Code::KeyX => VK_X,
            Code::KeyY => VK_Y,
            Code::KeyZ => VK_Z,
            Code::Digit0 => VK_0,
            Code::Digit1 => VK_1,
            Code::Digit2 => VK_2,
            Code::Digit3 => VK_3,
            Code::Digit4 => VK_4,
            Code::Digit5 => VK_5,
            Code::Digit6 => VK_6,
            Code::Digit7 => VK_7,
            Code::Digit8 => VK_8,
            Code::Digit9 => VK_9,
            Code::Equal => VK_OEM_PLUS,
            Code::Comma => VK_OEM_COMMA,
            Code::Minus => VK_OEM_MINUS,
            Code::Period => VK_OEM_PERIOD,
            Code::Semicolon => VK_OEM_1,
            Code::Slash => VK_OEM_2,
            Code::Backquote => VK_OEM_3,
            Code::BracketLeft => VK_OEM_4,
            Code::Backslash => VK_OEM_5,
            Code::BracketRight => VK_OEM_6,
            Code::Quote => VK_OEM_7,
            Code::Backspace => VK_BACK,
            Code::Tab => VK_TAB,
            Code::Space => VK_SPACE,
            Code::Enter => VK_RETURN,
            Code::CapsLock => VK_CAPITAL,
            Code::Escape => VK_ESCAPE,
            Code::PageUp => VK_PRIOR,
            Code::PageDown => VK_NEXT,
            Code::End => VK_END,
            Code::Home => VK_HOME,
            Code::ArrowLeft => VK_LEFT,
            Code::ArrowUp => VK_UP,
            Code::ArrowRight => VK_RIGHT,
            Code::ArrowDown => VK_DOWN,
            Code::PrintScreen => VK_SNAPSHOT,
            Code::Insert => VK_INSERT,
            Code::Delete => VK_DELETE,
            Code::F1 => VK_F1,
            Code::F2 => VK_F2,
            Code::F3 => VK_F3,
            Code::F4 => VK_F4,
            Code::F5 => VK_F5,
            Code::F6 => VK_F6,
            Code::F7 => VK_F7,
            Code::F8 => VK_F8,
            Code::F9 => VK_F9,
            Code::F10 => VK_F10,
            Code::F11 => VK_F11,
            Code::F12 => VK_F12,
            Code::F13 => VK_F13,
            Code::F14 => VK_F14,
            Code::F15 => VK_F15,
            Code::F16 => VK_F16,
            Code::F17 => VK_F17,
            Code::F18 => VK_F18,
            Code::F19 => VK_F19,
            Code::F20 => VK_F20,
            Code::F21 => VK_F21,
            Code::F22 => VK_F22,
            Code::F23 => VK_F23,
            Code::F24 => VK_F24,
            Code::NumLock => VK_NUMLOCK,
            Code::Numpad0 => VK_NUMPAD0,
            Code::Numpad1 => VK_NUMPAD1,
            Code::Numpad2 => VK_NUMPAD2,
            Code::Numpad3 => VK_NUMPAD3,
            Code::Numpad4 => VK_NUMPAD4,
            Code::Numpad5 => VK_NUMPAD5,
            Code::Numpad6 => VK_NUMPAD6,
            Code::Numpad7 => VK_NUMPAD7,
            Code::Numpad8 => VK_NUMPAD8,
            Code::Numpad9 => VK_NUMPAD9,
            Code::NumpadAdd => VK_ADD,
            Code::NumpadDecimal => VK_DECIMAL,
            Code::NumpadDivide => VK_DIVIDE,
            Code::NumpadEnter => VK_RETURN,
            Code::NumpadEqual => VK_E,
            Code::NumpadMultiply => VK_MULTIPLY,
            Code::NumpadSubtract => VK_SUBTRACT,
            Code::ScrollLock => VK_SCROLL,
            Code::AudioVolumeDown => VK_VOLUME_DOWN,
            Code::AudioVolumeUp => VK_VOLUME_UP,
            Code::AudioVolumeMute => VK_VOLUME_MUTE,
            Code::MediaPlay => VK_PLAY,
            Code::MediaPause => VK_PAUSE,
            Code::MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
            Code::MediaStop => VK_MEDIA_STOP,
            Code::MediaTrackNext => VK_MEDIA_NEXT_TRACK,
            Code::MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
            _ => return None,
        })
    }
}