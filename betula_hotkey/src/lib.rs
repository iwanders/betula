use betula_core::BetulaError;
pub mod nodes;

pub type HotkeyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

enum HotkeyTask {}

// use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

pub struct HotkeyRunner {
    thread: Option<std::thread::JoinHandle<()>>,
    running: std::sync::Arc<AtomicBool>,
}

impl HotkeyRunner {
    pub fn new() -> Result<HotkeyInterface, BetulaError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let t_running = running.clone();
        let (sender, receiver) = channel::<HotkeyTask>();

        let thread = Some(std::thread::spawn(move || {
            let _ = receiver;
            while t_running.load(std::sync::atomic::Ordering::Relaxed) {
                while let Ok(v) = receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    match v {}
                }
            }
        }));
        let runner = Arc::new(HotkeyRunner { thread, running });

        Ok(HotkeyInterface { sender, runner })
    }
}

impl Drop for HotkeyRunner {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let t = self.thread.take();
        t.unwrap().join().expect("join should succeed");
    }
}

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct HotkeyInterface {
    sender: Sender<HotkeyTask>,
    // dead code allowed, it contains the execution thread.
    #[allow(dead_code)]
    runner: Arc<HotkeyRunner>,
}

impl std::fmt::Debug for HotkeyInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "HotkeyInterface<{:?}>", Arc::as_ptr(&self.runner))
    }
}
impl std::cmp::PartialEq for HotkeyInterface {
    fn eq(&self, other: &HotkeyInterface) -> bool {
        Arc::as_ptr(&self.runner) == Arc::as_ptr(&other.runner)
    }
}
