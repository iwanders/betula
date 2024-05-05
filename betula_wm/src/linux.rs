use crate::WindowFocusError;
pub type BackendType = X11FocusHandler;
pub type CacheKey = std::ffi::c_ulong;

// Okay so this is kinda tricky and requires traversing things;
// https://stackoverflow.com/q/151407
// Found an implementation here:
// https://github.com/david-cattermole/timetracker/blob/2591383f45667e7be0378c7abffcad62c3a914aa/recorder-bin/src/linux_x11.rs
// MIT license, so large parts copied.

use x11_dl::xlib::{self, Xlib, _XDisplay};

use std::ffi::{c_int, c_long, c_uchar, c_uint, c_ulong, c_void};
use std::sync::Mutex;

pub type ProcessID = c_uint;

/// The error states that X11 can be in.
#[derive(Debug, Copy, Clone, PartialEq)]
enum XError {
    Failure,
    Success,
}

// A function that is called to handle errors, when X11 fails.
extern "C" fn handle_error_callback(
    _display_ptr: *mut xlib::Display,
    error_ptr: *mut xlib::XErrorEvent,
) -> c_int {
    // warn!("X11 error detected.");
    if !error_ptr.is_null() {
        let xerror_data = unsafe { *error_ptr };
        // debug!("X11 error data: {:?}", xerror_data);
        if xerror_data.error_code == xlib::BadWindow {
            // debug!("BadWindow: Window does not exist.");
            unsafe {
                X11_ERROR = XError::Failure;
            }
        }
    }

    1
}

/// The global error status of X11.
static mut X11_ERROR: XError = XError::Success;

struct Handler {
    instance: Xlib,
    display: *mut _XDisplay,
}

impl Handler {
    fn get_window_id_with_focus(&self) -> Result<u64, WindowFocusError> {
        unsafe {
            let mut window: std::ffi::c_ulong = 0;
            let mut ret: std::ffi::c_int = 0;
            (self.instance.XGetInputFocus)(self.display, &mut window, &mut ret);
            Ok(window)
        }
    }

    fn get_process_id_property_id(&self) -> Result<xlib::Atom, WindowFocusError> {
        let atom_name = std::ffi::CStr::from_bytes_with_nul(b"_NET_WM_PID\0")?;
        let atom_name_ptr = atom_name.as_ptr();
        let only_if_exists = 1 as std::ffi::c_int;
        let property_id: xlib::Atom =
            unsafe { (self.instance.XInternAtom)(self.display, atom_name_ptr, only_if_exists) };
        Ok(property_id)
    }

    fn get_process_id_from_window_id(
        &self,
        window_id: c_ulong,
        property_id: xlib::Atom,
    ) -> ProcessID {
        let long_offset = 0 as c_long;
        let long_length = 1 as c_long;
        let delete = xlib::False as c_int;
        let req_type = xlib::XA_CARDINAL;

        let mut actual_type_return = 0 as c_ulong;
        let mut actual_format_return = 0 as c_int;
        let mut nitems_return = 0 as c_ulong;
        let mut bytes_after_return = 0 as c_ulong;
        let mut prop_return_ptr: *mut c_uchar = std::ptr::null_mut();

        // https://tronche.com/gui/x/xlib/window-information/XGetWindowProperty.html
        let status: c_int = unsafe {
            (self.instance.XGetWindowProperty)(
                self.display,
                window_id,
                property_id,
                long_offset,
                long_length,
                delete,
                req_type,
                &mut actual_type_return,
                &mut actual_format_return,
                &mut nitems_return,
                &mut bytes_after_return,
                &mut prop_return_ptr,
            )
        };

        let mut process_id = 0;
        if status == (xlib::Success as i32) {
            if actual_type_return == xlib::XA_CARDINAL && actual_format_return == 32 {
                process_id = unsafe { *(prop_return_ptr as *mut ProcessID) };
            }
            unsafe { (self.instance.XFree)(prop_return_ptr as *mut c_void) };
        }

        process_id
    }

    fn get_process_id_from_window_tree(
        &self,
        start_window_id: c_ulong,
        property_id: xlib::Atom,
    ) -> ProcessID {
        let mut parent_window_id = start_window_id;
        let mut root_window_id = 0 as c_ulong;
        let mut child_window_ids = std::ptr::null_mut::<c_ulong>();
        let mut child_count = 0 as c_uint;
        let mut process_id = 0;

        while parent_window_id != root_window_id {
            let window_id = parent_window_id;

            // We install a error callback to stop the program from
            // exiting when an invalid window_id is used. Instead we just
            // pretend it didn't happen.
            unsafe {
                (self.instance.XSetErrorHandler)(Some(handle_error_callback));
            }
            let status = unsafe {
                (self.instance.XQueryTree)(
                    self.display,
                    window_id,
                    &mut root_window_id,
                    &mut parent_window_id,
                    &mut child_window_ids,
                    &mut child_count,
                )
            };

            unsafe {
                (self.instance.XSetErrorHandler)(None);
            }

            unsafe {
                if X11_ERROR == XError::Failure {
                    // warn!("XQueryTree failed for window_id: {}", window_id);
                    // Reset the global variable so we don't come here
                    // again when nothing has failed.
                    X11_ERROR = XError::Success;
                    break;
                }
            }

            // https://docs.rs/x11-dl/2.21.0/x11_dl/xlib/constant.Success.html
            // pub const Success: c_uchar = 0;
            // Yet docs for XQueryTree state:
            // XQueryTree() returns zero if it fails and nonzero if it succeeds
            if status != 0 {
                unsafe {
                    (self.instance.XFree)(child_window_ids as *mut c_void);
                };
            }
            // let num_properties = list_window_properties(display_ptr, window_id);
            process_id = self.get_process_id_from_window_id(window_id, property_id);
            if process_id > 0 {
                break;
            }
        }

        process_id
    }

    fn focussed_window_pid(&self) -> Result<(CacheKey, u32), WindowFocusError> {
        let window_id = self.get_window_id_with_focus()?;
        let property_id = self.get_process_id_property_id()?;
        let process_id = self.get_process_id_from_window_tree(window_id, property_id);
        Ok((window_id, process_id))
    }

    fn focussed_window_id(&self) -> Result<CacheKey, WindowFocusError> {
        let window_id = self.get_window_id_with_focus()?;
        Ok(window_id)
    }

    /*
    fn cursor_position(&self) -> Result<CursorPosition, WindowFocusError>  {
        unsafe {
            let count = (self.instance.XScreenCount)(self.display);
            if count < 1 {
                return Err("found less than one screen".into());
            }
            let root_window = (self.instance.XRootWindow)(self.display, 0);
            if root_window == 0 {
                return Err("could not get root window".into());
            }
            let mut root_return: xlib::Window = 0;
            let mut child_return: xlib::Window = 0;
            let mut root_x_return: c_int = 0;
            let mut root_y_return: c_int = 0;
            let mut win_x_return: c_int = 0;
            let mut win_y_return: c_int = 0;
            let mut mask_return: c_uint = 0;
            let res = (self.instance.XQueryPointer)(self.display, root_window,
                    &mut root_return,
                    &mut child_return,
                    &mut root_x_return,
                    &mut root_y_return,
                    &mut win_x_return,
                    &mut win_y_return,
                    &mut mask_return);
            // We always retrieve the root, so root and win retrieval are identical.
            if res > 0 {
                Ok(CursorPosition{
                    x: win_x_return,
                    y: win_y_return,
                })
            } else {
                Err("failed to retriever cursor position".into())
            }
        }
    }
    */
}

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
    fn setup(&self) -> Result<(), WindowFocusError> {
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

    pub fn process_name(&self, pid: u32) -> Result<String, WindowFocusError> {
        let path = std::fs::read_link(format!("/proc/{pid}/exe"))?;
        Ok(path
            .into_os_string()
            .into_string()
            .map_err(|e| format!("failed to convert link to string {e:?}"))?)
    }

    pub fn process_id(&self) -> Result<u32, WindowFocusError> {
        self.setup()?;
        let locked = self
            .handle
            .lock()
            .map_err(|_| format!("failed to lock mutex"))?;
        if let Some(v) = locked.as_ref() {
            return Ok(v.focussed_window_pid()?.1);
        }
        Err("failed to obtain focussed process id".into())
    }

    pub fn cache_key(&self) -> Result<CacheKey, WindowFocusError> {
        self.setup()?;
        let locked = self
            .handle
            .lock()
            .map_err(|_| format!("failed to lock mutex"))?;
        if let Some(v) = locked.as_ref() {
            return v.focussed_window_id();
        }
        Err("failed to obtain focussed process id".into())
    }

    pub fn cache(&self) -> Result<(CacheKey, String), WindowFocusError> {
        self.setup()?;
        let locked = self
            .handle
            .lock()
            .map_err(|_| format!("failed to lock mutex"))?;
        if let Some(v) = locked.as_ref() {
            let (cache_key, pid) = v.focussed_window_pid()?;
            return Ok((cache_key, self.process_name(pid)?));
        }
        Err("failed to obtain focussed process id".into())
    }

    /*
    pub fn cursor_position(&self) -> Result<CursorPosition, WindowFocusError> {
        self.setup()?;
        let locked = self
            .handle
            .lock()
            .map_err(|_| format!("failed to lock mutex"))?;
        if let Some(v) = locked.as_ref() {
            return v.cursor_position();
        }
        Err("failed to retrieve cursor position".into())
    }
    */
}
