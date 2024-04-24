pub type WindowFocusError = Box<dyn std::error::Error + Send + Sync + 'static>;
use serde::{Serialize, Deserialize};

pub mod nodes;

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

#[derive(Debug, Default)]
struct WindowFocus {
    backend: backend::BackendType,
    cache: Option<(backend::CacheKey, String)>,
}
impl WindowFocus {
    pub fn new() -> Self {
        Self::default()
    }

    fn raw_process_id(&self) -> Result<u32, WindowFocusError> {
        self.backend.process_id()
    }

    fn raw_process_name(&self, pid: u32) -> Result<String, WindowFocusError> {
        self.backend.process_name(pid)
    }

    pub fn process_name(&mut self) -> Result<String, WindowFocusError> {
        let id = self.backend.cache_key()?;
        if let Some((cached_id, cached_name)) = self.cache.as_ref() {
            if *cached_id == id {
                return Ok(cached_name.clone());
            } else {
                self.cache = None;
            }
        }

        let cacheable = self.backend.cache()?;
        let name = cacheable.1.clone();
        self.cache = Some(cacheable);
        Ok(name)
    }

    pub fn cursor_position(&self) -> Result<CursorPosition, WindowFocusError> {
        self.backend.cursor_position()
    }
}


/// Structure to represent a cursor position.
///
/// Windows: 0,0 is top left of primary, top right is 1919,0, bottom right is 1919,1079. Left monitor (non primary) is
/// -1920,0 top left and -1920,1079 bottom left.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CursorPosition{
    pub x: i32,
    pub y: i32,
}

pub fn main_test() {
    let mut helper = WindowFocus::new();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        {
            let pid = helper.raw_process_id().unwrap();
            let name = helper.raw_process_name(pid).unwrap();
            println!("{pid} -> {name}");
        }
        if let Ok(n) = helper.process_name() {
            println!("name: {n}");
        }
        if let Ok(p) = helper.cursor_position() {
            println!("cursor possition: {p:?}");
        }
    }
}

/// Register enigo nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {
    ui_support
        .add_node_default_with_config::<nodes::WindowFocusNode, nodes::WindowFocusNodeConfig>();
}
