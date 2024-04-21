use serde::{Deserialize, Serialize};

pub type WindowFocusError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

#[derive(Debug, Default)]
struct WindowFocus {
    cache: Option<(u32, String)>,
}
impl WindowFocus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn raw_focussed_process_id(&self) -> Result<u32, WindowFocusError> {
        backend::BackendType::focussed_process_id()
    }

    pub fn raw_process_name(&self, pid: u32) -> Result<String, WindowFocusError> {
        backend::BackendType::process_name(pid)
    }

    pub fn process_name(&mut self) -> Result<String, WindowFocusError> {
        let id = backend::BackendType::focussed_process_id()?;
        if let Some((cached_id, cached_name)) = self.cache.as_ref() {
            if *cached_id == id {
                return Ok(cached_name.clone());
            } else {
                self.cache = None;
            }
        }
        let pid = self.raw_focussed_process_id()?;
        let name = self.raw_process_name(pid)?;
        self.cache = Some((pid, name.clone()));
        Ok(name)
    }
}

pub fn main_test() {
    let mut helper = WindowFocus::new();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        if let Ok(n) = helper.process_name() {
            println!("{n}");
        }
    }
}
