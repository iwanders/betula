// All rpc crates I found pull in hundreds of dependencies, so this here just makes one that's trivial.
// Only on loopback / trusted networks: No security.
// No state / persistence in connection.
//   Connect, finish procedure, disconnect.
//   Server has state, it can be cleared.
// Trivial interface.
// We'll probably assume there's only a single client for now.

use crate::OverlayError;
use screen_overlay::{Overlay, OverlayConfig, OverlayHandle, VisualId};
use serde::{Deserialize, Serialize};

use std::cell::RefCell;
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OverlayDaemonConfig {
    pub bind: std::net::SocketAddr,
}
pub struct OverlayServer {
    listener: RefCell<TcpListener>,
    handle: RefCell<OverlayHandle>,
}

pub mod instructions {
    use super::*;
    use screen_overlay::egui::Color32;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    pub struct Text {
        #[serde(default)]
        pub position: (f32, f32),

        #[serde(default)]
        pub size: (f32, f32),

        #[serde(default)]
        pub font_size: f32,

        #[serde(default)]
        pub text_color: Color32,

        #[serde(default)]
        pub fill_color: Color32,

        #[serde(default)]
        pub text: String,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
enum Payload {
    #[default]
    Empty,
    Id(VisualId),
    Text(instructions::Text),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
enum Command {
    #[default]
    Hello,
    Add,
    RemoveAllElements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OverlayRequest {
    command: Command,
    payload: Payload,
}
impl OverlayRequest {
    fn to_response(&self) -> OverlayResponse {
        OverlayResponse {
            command: self.command,
            payload: Payload::Empty,
        }
    }
    fn reply_with(&self, payload: Payload) -> OverlayResponse {
        OverlayResponse {
            command: self.command,
            payload,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OverlayResponse {
    command: Command,
    payload: Payload,
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
            Command::Hello => Ok(req.to_response()),
            Command::RemoveAllElements => {
                overlay.remove_all_elements();
                Ok(req.to_response())
            }
            Command::Add => match &req.payload {
                Payload::Text(text) => {
                    use screen_overlay::PositionedElements;
                    let text_string = text.text.clone();

                    let font_size = text.font_size;
                    let text_color = text.text_color;
                    let drawable = PositionedElements::new()
                        .fixed_pos(egui::pos2(text.position.0, text.position.1))
                        .default_size(egui::vec2(text.size.0, text.size.1))
                        .fill(text.fill_color)
                        .add_closure(move |ui| {
                            let text = egui::widget_text::RichText::new(&text_string)
                                .size(font_size)
                                .color(text_color);
                            ui.label(text);
                        });

                    let text_token = overlay.add_drawable(drawable.into());
                    let text_id = text_token.into_id();
                    return Ok(req.reply_with(Payload::Id(text_id)));
                }
                payload => {
                    return Err(format!("got incorrect payload: {:?}", payload).into());
                }
            },
        }
    }
    pub fn service(&mut self) -> Result<(), OverlayError> {
        for stream in self.listener.borrow_mut().incoming() {
            match stream {
                Ok(s) => {
                    // do something with the TcpStream
                    let req: OverlayRequest = single_value_from_stream(&s)?;
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

        serde_json::to_writer(&s, &req)?;
        let resp: OverlayResponse = serde_json::from_reader(&s)?;
        Ok(resp)
    }

    fn send_empty(&self, command: &Command) -> Result<(), OverlayError> {
        let resp = self.request(&OverlayRequest {
            command: *command,
            payload: Payload::Empty,
        })?;
        if resp.command == *command {
            return Ok(());
        } else {
            Err(format!("got unexpected instruction back {:?}", resp.command).into())
        }
    }
    fn send(&self, command: &Command, payload: Payload) -> Result<Payload, OverlayError> {
        let resp = self.request(&OverlayRequest {
            command: *command,
            payload: payload,
        })?;
        if resp.command == *command {
            return Ok(resp.payload);
        } else {
            Err(format!("got unexpected instruction back {:?}", resp.command).into())
        }
    }

    pub fn hello(&self) -> Result<(), OverlayError> {
        self.send_empty(&Command::Hello)
    }

    pub fn remove_all_elements(&self) -> Result<(), OverlayError> {
        self.send_empty(&Command::RemoveAllElements)
    }

    pub fn add_text(&self, text: instructions::Text) -> Result<VisualId, OverlayError> {
        let resp = self.send(&Command::Add, Payload::Text(text))?;
        if let Payload::Id(z) = resp {
            Ok(z)
        } else {
            Err(format!("got incorrect payload for add_text {:?}", resp).into())
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
