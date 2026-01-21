// All rpc crates I found pull in hundreds of dependencies, so this here just makes one that's trivial.
// Only on loopback / trusted networks: No security.
// No state / persistence in connection.
//   Connect, finish procedure, disconnect.
//   Server has state, it can be cleared.
// Trivial interface.
// We'll probably assume there's only a single client for now.

use crate::OverlayError;
use screen_overlay::{Overlay, OverlayConfig, OverlayHandle};
use serde::{Deserialize, Serialize};

use std::cell::RefCell;
use std::io::Write as _;
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OverlayDaemonConfig {
    pub bind: std::net::SocketAddr,
}
pub struct OverlayServer {
    listener: RefCell<TcpListener>,
    handle: RefCell<OverlayHandle>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
enum Instruction {
    Hello,
    ClearAll,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OverlayRequest {
    command: Instruction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OverlayResponse {
    command: Instruction,
}

// serde_json::from_reader
//  > If the stream does not end, such as in the case of a persistent socket connection, this function will not return.
//  > It is possible instead to deserialize from a prefix of an input stream without looking for EOF by managing your own Deserializer.
fn single_value_from_stream<T: serde::de::DeserializeOwned, S: std::io::Read>(
    stream: S,
) -> Result<T, serde_json::Error> {
    serde_json::Deserializer::from_reader(stream)
        .into_iter::<T>()
        .next()
        .unwrap() // we'll either get a value, or we'll get a parse error? Sounds legit?
}

impl OverlayServer {
    pub fn new(config: OverlayDaemonConfig, handle: OverlayHandle) -> Result<Self, OverlayError> {
        let listener = TcpListener::bind(config.bind)?;
        listener
            .set_nonblocking(true)
            .expect("Cannot set non-blocking");
        Ok(OverlayServer {
            listener: listener.into(),
            handle: handle.into(),
        })
    }

    fn process_request(&self, req: &OverlayRequest) -> Result<OverlayResponse, OverlayError> {
        let overlay = self.handle.borrow_mut();
        println!("process_request: {req:?}");
        match req.command {
            Instruction::Hello => Ok(OverlayResponse {
                command: Instruction::Hello,
            }),
            Instruction::ClearAll => todo!(),
        }
    }
    pub fn service(&mut self) -> Result<(), OverlayError> {
        for stream in self.listener.borrow_mut().incoming() {
            match stream {
                Ok(s) => {
                    // do something with the TcpStream
                    let req: OverlayRequest = single_value_from_stream(&s)?;
                    println!("req: {req:?}");
                    let resp = self.process_request(&req)?;
                    serde_json::to_writer(&s, &resp)?;
                    // Dropping the stream closes it.
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    // wait_for_fd();
                    continue;
                }
                Err(e) => return Err(format!("something went wrong {e:?}").into()),
            }
        }
        Ok(())
    }
}

pub struct OverlayClient {
    config: OverlayDaemonConfig,
}
impl OverlayClient {
    pub fn new(config: OverlayDaemonConfig) -> Self {
        Self { config }
    }
    fn request(&self, req: &OverlayRequest) -> Result<OverlayResponse, OverlayError> {
        let s = TcpStream::connect(self.config.bind)?;

        println!("sending req");
        serde_json::to_writer(&s, &req)?;
        let resp: OverlayResponse = serde_json::from_reader(&s)?;
        Ok(resp)
    }
    pub fn hello(&self) -> Result<(), OverlayError> {
        let resp = self.request(&OverlayRequest {
            command: Instruction::Hello,
        })?;
        if resp.command == Instruction::Hello {
            return Ok(());
        } else {
            Err(format!("got unexpected instruction back {:?}", resp.command).into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_overlay_interaction() -> Result<(), OverlayError> {
        let (width, height) = (1920.0, 1080.0);
        let (x, y) = (0.0, 0.0);
        let config = OverlayConfig::new()
            .with_size([width, height])
            .with_position([x, y])
            .with_central_panel_fill(screen_overlay::egui::Color32::TRANSPARENT);
        let overlay = Overlay::new(config);
        let overlay = OverlayHandle::new(overlay);

        let daemon_config = OverlayDaemonConfig {
            bind: "127.0.0.1:1337".parse().unwrap(),
        };
        let mut server = OverlayServer::new(daemon_config, overlay).unwrap();

        let z = std::thread::spawn(move || {
            use std::time::Duration;
            let start_time = std::time::Instant::now();
            loop {
                let from_start = start_time.elapsed().as_secs_f64();
                if from_start >= 1.0 {
                    break;
                }
                let _ = server.service();
                std::thread::sleep(Duration::from_millis(1));
            }
        });

        let client = OverlayClient::new(daemon_config);
        assert_eq!(client.hello()?, ());
        Ok(())
    }
}
