use serde::{Deserialize, Serialize};

pub mod nodes;

use enigo::Enigo;

use enigo::agent::Agent;
use enigo::agent::Token;

enum EnigoTask {
    // SetDelay(u32),
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
            while t_running.load(std::sync::atomic::Ordering::Relaxed) {
                while let Ok(v) = receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    let mut locked = enigo.lock().unwrap();
                    match v {
                        // EnigoTask::SetDelay(d) => {
                        // locked.set_delay(d);
                        // },
                        EnigoTask::Tokens(z) => {
                            //// Don't really know how to handle this Result.
                            for t in z {
                                let _ = locked
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
}
impl std::fmt::Debug for EnigoBlackboard {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Enigo")
    }
}

/// Register enigo nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {
    ui_support.add_node_default::<nodes::EnigoInstanceNode>();
    ui_support.add_node_default_with_config::<nodes::EnigoNode, nodes::EnigoNodeConfig>();
    ui_support.add_value_default_named::<EnigoBlackboard>("Enigo");
}
