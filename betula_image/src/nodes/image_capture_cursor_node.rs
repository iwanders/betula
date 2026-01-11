use betula_core::node_prelude::*;

use super::ImageCaptureNode;
use crate::{Image, ImageCursor};
use screen_capture::ThreadedCapturer;

use betula_common::callback::CallbacksBlackboard;
use betula_enigo::EnigoBlackboard;

use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicI32, AtomicUsize};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct EnigoData {
    counter: AtomicUsize,
    cursor_position: (AtomicI32, AtomicI32),
    cursor_offset: Arc<(AtomicI32, AtomicI32)>,
}

#[derive(Debug, Default, Clone)]
struct FullData {
    image_cursor: ImageCursor,
    time: f64,
    duration: f64,
}

type ImageCursorData = Arc<Mutex<Option<FullData>>>;

pub struct ImageCaptureCursorNode {
    enigo: Input<EnigoBlackboard>,
    output: Output<ImageCursor>,
    callbacks: CallbacksBlackboard<ImageCursor>,
    output_cb: Output<CallbacksBlackboard<ImageCursor>>,
    node: ImageCaptureNode,
    setup_done: bool,
    data: ImageCursorData,
}
impl Default for ImageCaptureCursorNode {
    fn default() -> ImageCaptureCursorNode {
        let data = Arc::new(Mutex::new(None));
        let callbacks = CallbacksBlackboard::<ImageCursor>::new();
        Self {
            enigo: Default::default(),
            callbacks,
            output: Default::default(),
            node: Default::default(),
            output_cb: Default::default(),
            setup_done: false,
            data,
        }
    }
}

impl std::fmt::Debug for ImageCaptureCursorNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageCaptureCursorNode")
    }
}

impl Node for ImageCaptureCursorNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let c = self
            .node
            .capture
            .get_or_insert_with(|| ThreadedCapturer::new(self.node.config.capture.clone()));
        // let info = c.latest();

        let interface = self.enigo.get()?;
        if !self.setup_done {
            if let Some(enigo) = interface.enigo() {
                let cursor_offset = interface.cursor_offset().unwrap();

                let data_block = Arc::new(EnigoData {
                    counter: 0.into(),
                    cursor_position: (0.into(), 0.into()),
                    cursor_offset,
                });
                let post_data_block = Arc::clone(&data_block);

                // Now we have both enigo, and the cursor offset, we can craft the callbacks
                let pre_callback = Arc::new(move |counter: usize| {
                    // Store the data!
                    data_block.counter.store(counter, Relaxed);
                    // get the cursor position;
                    let location = {
                        use betula_enigo::enigo::Mouse;
                        let locked = enigo.lock().expect("should not be poisoned");
                        locked.location().unwrap_or((0, 0))
                    };
                    // Convert that using the offsets.
                    data_block.cursor_position.0.store(
                        location.0 - data_block.cursor_offset.0.load(Relaxed),
                        Relaxed,
                    );
                    data_block.cursor_position.1.store(
                        location.1 - data_block.cursor_offset.1.load(Relaxed),
                        Relaxed,
                    );
                    // println!("data_block: {data_block:?}");
                });
                c.set_pre_callback(pre_callback);

                // Now, we can craft the post callback.
                let data = Arc::clone(&self.data);
                let cb = self
                    .callbacks
                    .callbacks()
                    .map(|v| v.clone())
                    .expect("callbacks is always populated here");
                let post_callback =
                    Arc::new(move |capture_info: screen_capture::capturer::CaptureInfo| {
                        let (cx, cy) = (
                            post_data_block.cursor_position.0.load(Relaxed),
                            post_data_block.cursor_position.1.load(Relaxed),
                        );
                        let time = capture_info
                            .time
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs_f64();
                        let duration = capture_info.duration.as_secs_f64();
                        if let Ok(info) = capture_info.result {
                            let image_cursor = ImageCursor {
                                image: crate::Image::new(info),
                                cursor: betula_enigo::CursorPosition { x: cx, y: cy },
                                time,
                                counter: capture_info.counter,
                            };
                            let ic = image_cursor.clone();
                            let full_data = FullData {
                                image_cursor,
                                time,
                                duration,
                            };
                            // Finally, assign the full data.
                            {
                                let mut locked = data.lock().unwrap();
                                *locked = Some(full_data);
                            }
                            (cb).call(ic);
                        }
                    });
                c.set_post_callback(post_callback);

                self.setup_done = true;
            }
        }

        let full_data = {
            let locked = self.data.lock().unwrap();
            locked.as_ref().map(|z| (*z).clone())
        };
        let _ = self.output_cb.set(self.callbacks.clone());
        if let Some(full_data) = full_data {
            let _ = self.node.output.set(full_data.image_cursor.image.clone());
            let _ = self.output.set(full_data.image_cursor);
            let _ = self.node.output_time.set(full_data.time);
            let _ = self.node.output_duration.set(full_data.duration);
            Ok(ExecutionStatus::Success)
        } else {
            Ok(ExecutionStatus::Failure)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::output::<ImageCursor>("image_cursor"),
            Port::output::<Image>("image"),
            Port::output::<CallbacksBlackboard<ImageCursor>>("image_cursor_cb"),
            Port::output::<f64>("capture_time"),
            Port::output::<f64>("capture_duration"),
            Port::input::<EnigoBlackboard>("enigo"),
        ])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<ImageCursor>("image_cursor", Default::default())?;
        self.output_cb = interface
            .output::<CallbacksBlackboard<ImageCursor>>("image_cursor_cb", Default::default())?;
        self.node.setup_outputs(interface)
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.enigo = interface.input::<EnigoBlackboard>("enigo")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "image_capture_cursor".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        self.node.get_config()
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.node.set_config(config)
    }

    fn reset(&mut self) {
        self.setup_done = false;
        self.node.reset();
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for ImageCaptureCursorNode {
        fn ui_title(&self) -> String {
            "capture cursor 📷 ".to_owned()
        }

        fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut egui::Ui) -> UiConfigResponse {
            self.node.ui_config(ctx, ui)
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("image_capture_cursor".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
