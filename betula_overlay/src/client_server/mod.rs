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

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    impl Default for Text {
        fn default() -> Text {
            Text {
                position: (0.0, 0.0),
                size: (100.0, 100.0),
                font_size: 10.0,
                text_color: Color32::BLACK,
                fill_color: Color32::TRANSPARENT,
                text: "DummyText".to_owned(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Drawable {
    Text(instructions::Text),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
enum RequestCommand {
    #[default]
    Hello,
    Add(Drawable),
    Remove(VisualId),
    RemoveAllElements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OverlayRequest {
    command: RequestCommand,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
enum ResponseCommand {
    #[default]
    Hello,
    Add(VisualId),
    Remove(()),
    RemoveAllElements,
}
impl ResponseCommand {
    fn response_hello(&self) -> Result<(), OverlayError> {
        match self {
            ResponseCommand::Hello => Ok(()),
            other => Err(format!("incorrect response {other:?}").into()),
        }
    }

    fn response_add(&self) -> Result<VisualId, OverlayError> {
        match self {
            ResponseCommand::Add(id) => Ok(*id),
            other => Err(format!("incorrect response {other:?}").into()),
        }
    }

    fn response_remove(&self) -> Result<(), OverlayError> {
        match self {
            ResponseCommand::Remove(_) => Ok(()),
            other => Err(format!("incorrect response {other:?}").into()),
        }
    }

    fn response_remove_all(&self) -> Result<(), OverlayError> {
        match self {
            ResponseCommand::RemoveAllElements => Ok(()),
            other => Err(format!("incorrect response {other:?}").into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OverlayResponse {
    command: ResponseCommand,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BulkRequest(Vec<OverlayRequest>);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BulkResponse(Vec<OverlayResponse>);

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
        match &req.command {
            RequestCommand::Hello => Ok(OverlayResponse {
                command: ResponseCommand::Hello,
            }),
            RequestCommand::RemoveAllElements => {
                overlay.remove_all_elements();
                Ok(OverlayResponse {
                    command: ResponseCommand::RemoveAllElements,
                })
            }
            RequestCommand::Add(drawable) => match &drawable {
                Drawable::Text(text) => {
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
                    return Ok(OverlayResponse {
                        command: ResponseCommand::Add(text_id),
                    });
                }
            },
            RequestCommand::Remove(id) => {
                overlay.remove_element(*id);

                return Ok(OverlayResponse {
                    command: ResponseCommand::Remove(()),
                });
            }
        }
    }
    pub fn service(&mut self) -> Result<(), OverlayError> {
        for stream in self.listener.borrow_mut().incoming() {
            match stream {
                Ok(s) => {
                    // do something with the TcpStream
                    let requests: BulkRequest = single_value_from_stream(&s)?;
                    let mut responses: BulkResponse = Default::default();
                    for req in requests.0.iter() {
                        responses.0.push(self.process_request(&req)?);
                    }

                    serde_json::to_writer(&s, &responses)?;
                    // Dropping the stream closes it.
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    // wait_for_fd();
                    //continue;
                    return Ok(());
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
    fn request(&self, req: &[OverlayRequest]) -> Result<Vec<OverlayResponse>, OverlayError> {
        let s = TcpStream::connect(self.config.bind)?;
        let wire_request = BulkRequest(req.to_vec());
        serde_json::to_writer(&s, &wire_request)?;
        let resp: BulkResponse = serde_json::from_reader(&s)?;
        Ok(resp.0)
    }
    fn request_single(&self, req: &OverlayRequest) -> Result<OverlayResponse, OverlayError> {
        self.request(&[req.clone()])?
            .drain(..)
            .next()
            .ok_or("did not get a single response".into())
    }

    pub fn hello(&self) -> Result<(), OverlayError> {
        self.request_single(&OverlayRequest {
            command: RequestCommand::Hello,
        })?
        .command
        .response_hello()
    }

    pub fn remove_all_elements(&self) -> Result<(), OverlayError> {
        self.request_single(&OverlayRequest {
            command: RequestCommand::RemoveAllElements,
        })?
        .command
        .response_remove_all()
    }

    pub fn remove(&self, id: VisualId) -> Result<(), OverlayError> {
        self.request_single(&OverlayRequest {
            command: RequestCommand::Remove(id),
        })?
        .command
        .response_remove()
    }
    pub fn add_text(&self, text: instructions::Text) -> Result<VisualId, OverlayError> {
        self.request_single(&OverlayRequest {
            command: RequestCommand::Add(Drawable::Text(text)),
        })?
        .command
        .response_add()
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
