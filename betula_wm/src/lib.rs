pub type WindowFocusError = Box<dyn std::error::Error + Send + Sync + 'static>;
use serde::{Deserialize, Serialize};

pub mod nodes;

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

#[derive(Debug, Default)]
struct WindowFocusRetriever {
    backend: backend::BackendType,
    cache: Option<(backend::CacheKey, String)>,
}
impl WindowFocusRetriever {
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
}

#[derive(Debug, Default)]
struct CursorPositionRetriever {
    backend: backend::BackendType,
}
impl CursorPositionRetriever {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cursor_position(&self) -> Result<CursorPosition, WindowFocusError> {
        self.backend.cursor_position()
    }
}

/// Structure to represent a cursor position.
///
/// For two 1080p monitors, side by side, right one being primary:
///
/// Windows: 0,0 is top left of primary, top right is 1919,0, bottom right is 1919,1079. Left monitor (non primary) is
/// -1920,0 top left and -1920,1079 bottom left.
/// Linux: top left is 0,0, top right is 3839,0, bottom right is 3839,1070
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

pub fn main_test() {
    let mut window_focus = WindowFocusRetriever::new();
    let mut cursor_position = CursorPositionRetriever::new();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        {
            let pid = window_focus.raw_process_id().unwrap();
            let name = window_focus.raw_process_name(pid).unwrap();
            println!("{pid} -> {name}");
        }
        if let Ok(n) = window_focus.process_name() {
            println!("name: {n}");
        }
        if let Ok(p) = cursor_position.cursor_position() {
            println!("cursor position: {p:?}");
        }
    }
}

/// Register enigo nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {
    ui_support
        .add_node_default_with_config::<nodes::WindowFocusNode, nodes::WindowFocusNodeConfig>();
    ui_support
        .add_node_default_with_config::<nodes::CursorPositionNode, nodes::CursorPositionNodeConfig>(
        );
}
