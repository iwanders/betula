use serde::{Deserialize, Serialize};

pub mod nodes;

use enigo::Enigo;

use enigo::agent::Agent;
use enigo::agent::Token;

mod preset;
pub use preset::{load_preset_directory, EnigoPreset};

enum EnigoTask {
    SetAbsolutePosOffset(i32, i32),
    Tokens(Vec<Token>),
}

// use std::cell::RefCell;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

pub struct EnigoRunner {
    thread: Option<std::thread::JoinHandle<()>>,
    running: std::sync::Arc<AtomicBool>,
}

impl EnigoRunner {
    pub fn new() -> Result<EnigoInterface, enigo::NewConError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let cursor_offset: std::sync::Arc<(AtomicI32, AtomicI32)> =
            std::sync::Arc::new((0i32.into(), 0i32.into()));
        let t_running = Arc::clone(&running);
        let t_cursor_offset = Arc::clone(&cursor_offset);
        let (sender, receiver) = channel::<EnigoTask>();
        let settings = enigo::Settings {
            release_keys_when_dropped: true,
            ..Default::default()
        };

        let enigo = Arc::new(Mutex::new(Enigo::new(&settings)?));
        let enigo_t = Arc::clone(&enigo);
        let thread = Some(std::thread::spawn(move || {
            let enigo = enigo_t;
            let _ = receiver;
            let position_offset = t_cursor_offset;
            while t_running.load(Relaxed) {
                while let Ok(v) = receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    let mut locked = enigo.lock().unwrap();
                    match v {
                        EnigoTask::SetAbsolutePosOffset(x, y) => {
                            position_offset.0.store(x, Relaxed);
                            position_offset.1.store(y, Relaxed);
                        }
                        EnigoTask::Tokens(z) => {
                            for mut t in z {
                                if let Token::MoveMouse(x, y, coordinate) = &mut t {
                                    if *coordinate == enigo::Coordinate::Abs {
                                        *x += position_offset.0.load(Relaxed);
                                        *y += position_offset.1.load(Relaxed);
                                    }
                                }
                                //// Don't really know how to handle this Result, lets panic?
                                locked
                                    .execute(&t)
                                    .expect(&format!("failed to execute {t:?}"));
                            }
                        }
                    }
                }
            }
        }));

        let runner = Arc::new(EnigoRunner { thread, running });

        Ok(EnigoInterface {
            enigo,
            sender,
            runner,
            cursor_offset,
        })
    }
}

impl Drop for EnigoRunner {
    fn drop(&mut self) {
        self.running.store(false, Relaxed);
        let t = self.thread.take();
        t.unwrap().join().expect("join should succeed");
    }
}

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct EnigoInterface {
    sender: Sender<EnigoTask>,
    enigo: Arc<Mutex<Enigo>>,

    cursor_offset: std::sync::Arc<(AtomicI32, AtomicI32)>,

    // dead code allowed, it contains the execution thread.
    #[allow(dead_code)]
    runner: Arc<EnigoRunner>,
}

impl std::fmt::Debug for EnigoInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "EnigoInterface<{:?}>", Arc::as_ptr(&self.enigo))
    }
}
impl std::cmp::PartialEq for EnigoInterface {
    fn eq(&self, other: &EnigoInterface) -> bool {
        Arc::as_ptr(&self.enigo) == Arc::as_ptr(&other.enigo)
    }
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EnigoBlackboard {
    #[serde(skip)]
    pub interface: Option<EnigoInterface>,
}
impl EnigoBlackboard {
    pub fn execute(&self, tokens: &[Token]) -> Result<(), betula_core::BetulaError> {
        let interface = self
            .interface
            .as_ref()
            .ok_or(format!("no interface present in value"))?;
        let mut locked = interface.enigo.lock().expect("should not be poisoned");
        for t in tokens {
            locked.execute(t)?;
        }
        Ok(())
    }
    pub fn execute_async(&self, tokens: &[Token]) -> Result<(), betula_core::BetulaError> {
        let interface = self
            .interface
            .as_ref()
            .ok_or(format!("no interface present in value"))?;
        interface.sender.send(EnigoTask::Tokens(tokens.to_vec()))?;
        Ok(())
    }

    pub fn set_cursor_offset(&self, offset: (i32, i32)) -> Result<(), betula_core::BetulaError> {
        let interface = self
            .interface
            .as_ref()
            .ok_or(format!("no interface present in value"))?;
        interface
            .sender
            .send(EnigoTask::SetAbsolutePosOffset(offset.0, offset.1))?;
        Ok(())
    }
    pub fn cursor_location(&self) -> Result<CursorPosition, betula_core::BetulaError> {
        let interface = self
            .interface
            .as_ref()
            .ok_or(format!("no interface present in value"))?;
        let locked = interface.enigo.lock().expect("should not be poisoned");
        use enigo::Mouse;
        Ok(locked.location().map(|v| CursorPosition {
            x: v.0 - interface.cursor_offset.0.load(Relaxed),
            y: v.1 - interface.cursor_offset.1.load(Relaxed),
        })?)
    }
}
impl std::fmt::Debug for EnigoBlackboard {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Enigo")
    }
}

/// Structure to represent a cursor position.
///
/// For two 1080p monitors, side by side, right one being primary:
///
/// Windows: 0,0 is top left of primary, top right is 1919,0, bottom right is 1919,1079. Left monitor (non primary) is
/// -1920,0 top left and -1920,1079 bottom left.
/// Linux: top left is 0,0, top right is 3839,0, bottom right is 3839,1070
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

/// Register enigo nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    // ui_support.add_node_default::<nodes::EnigoInstanceNode>();
    ui_support
        .add_node_default_with_config::<nodes::EnigoInstanceNode, nodes::EnigoInstanceNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::EnigoNode, nodes::EnigoNodeConfig>();
    ui_support.add_node_default::<nodes::EnigoCursorNode>();
    ui_support.add_value_default_named::<EnigoBlackboard>("Enigo");
    ui_support.add_value_default_named::<CursorPosition>("Cursor");
}
