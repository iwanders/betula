use crate::HotkeyError;
use std::sync::Mutex;
use x11_dl::xlib::{self, Xlib, _XDisplay};

struct Handler {
    instance: Xlib,
    display: *mut _XDisplay,
}

impl Handler {}

#[derive(Default)]
pub struct X11FocusHandler {
    handle: Mutex<Option<Handler>>,
}

impl std::fmt::Debug for X11FocusHandler {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let locked = self.handle.lock().unwrap();
        write!(fmt, "X11FocusHandler<{}>", locked.is_some())
    }
}

impl X11FocusHandler {
    fn setup(&self) -> Result<(), HotkeyError> {
        let mut locked = self
            .handle
            .lock()
            .map_err(|_| format!("failed to lock mutex"))?;
        if locked.is_none() {
            let instance = xlib::Xlib::open()?;
            let display = unsafe { (instance.XOpenDisplay)(std::ptr::null()) };
            if display.is_null() {
                return Err("failed to retrieve display ptr".into());
            }
            *locked = Some(Handler { instance, display });
        }
        Ok(())
    }
}
