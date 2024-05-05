use serde::{Deserialize, Serialize};

pub mod nodes;

use enigo::Enigo;

use enigo::agent::Agent;
use enigo::agent::Token;

enum EnigoTask {
    SetAbsolutePosOffset(i32, i32),
    Tokens(Vec<Token>),
}

// use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

pub struct EnigoRunner {
    thread: Option<std::thread::JoinHandle<()>>,
    running: std::sync::Arc<AtomicBool>,
}

impl EnigoRunner {
    pub fn new() -> Result<EnigoInterface, enigo::NewConError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let t_running = running.clone();
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
            let mut position_offset = (0, 0);
            while t_running.load(std::sync::atomic::Ordering::Relaxed) {
                while let Ok(v) = receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    let mut locked = enigo.lock().unwrap();
                    match v {
                        EnigoTask::SetAbsolutePosOffset(x, y) => {
                            position_offset = (x, y);
                        }
                        EnigoTask::Tokens(z) => {
                            for mut t in z {
                                if let Token::MoveMouse(x, y, coordinate) = &mut t {
                                    if *coordinate == enigo::Coordinate::Abs {
                                        *x += position_offset.0;
                                        *y += position_offset.1;
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
        })
    }
}

impl Drop for EnigoRunner {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let t = self.thread.take();
        t.unwrap().join().expect("join should succeed");
    }
}

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct EnigoInterface {
    sender: Sender<EnigoTask>,
    enigo: Arc<Mutex<Enigo>>,

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
        Ok(locked
            .location()
            .map(|v| CursorPosition { x: v.0, y: v.1 })?)
    }
}
impl std::fmt::Debug for EnigoBlackboard {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Enigo")
    }
}

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
    ui_support.add_value_default_named::<CursorPosition>("EnigoCursorPosition");
}
