use windows::{
    core::PWSTR,
    Win32::Foundation::{HANDLE, HWND},
    Win32::System::ProcessStatus::EnumProcesses,
    Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_NATIVE,
        PROCESS_QUERY_LIMITED_INFORMATION,
    },
    Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

use crate::WindowFocusError;
type DWORD = u32;

pub type BackendType = WindowsFocusHandler;

#[derive(Default, Debug)]
pub struct WindowsFocusHandler {}
impl WindowsFocusHandler {
    // This doesn't actually get us any more than the direct process id from the handle.
    pub fn get_process_ids(&self) -> Result<Vec<u32>, WindowFocusError> {
        unsafe {
            // EnumProcesses returns how many bytes it wrote :/
            const MAX_PROCESS_COUNT: usize = 4096;
            let mut bytes_written: u32 = 0;
            let mut buffer: Vec<u32> = Vec::with_capacity(MAX_PROCESS_COUNT);
            let result = EnumProcesses(
                buffer.as_mut_ptr(),
                (MAX_PROCESS_COUNT * std::mem::size_of::<u32>()) as u32,
                &mut bytes_written,
            )?;

            let process_count = bytes_written as usize / std::mem::size_of::<u32>();
            buffer.set_len(process_count);
            Ok(buffer)
        }
    }

    pub fn process_name(&self, pid: u32) -> Result<String, WindowFocusError> {
        let h_process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };
        if h_process == HANDLE(0) {
            return Err(format!("failed to open process with pid {pid}").into());
        }
        unsafe {
            const CHARACTER_BUFFER_LENGTH: usize = 4096;
            let mut buffer = [0u16; CHARACTER_BUFFER_LENGTH];
            let mut size = CHARACTER_BUFFER_LENGTH as u32;
            let ptr = PWSTR::from_raw(buffer.as_mut_ptr());
            QueryFullProcessImageNameW(h_process, PROCESS_NAME_NATIVE, ptr, &mut size)?;
            // Succeeded, null termination is guaranteed according to the docs, so we can string convert this.
            Ok(ptr.to_string()?)
        }
    }

    pub fn focussed_process_id(&self) -> Result<u32, WindowFocusError> {
        unsafe {
            let fg: HWND = GetForegroundWindow();
            let mut out: DWORD = 0;
            GetWindowThreadProcessId(fg, Some(&mut out));
            Ok(out)
        }
    }
}
