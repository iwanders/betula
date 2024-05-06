use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::Image;

use crate::pattern_match::{load_patterns_directory, PatternEntry, PatternName};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageMatchNodeConfig {
    use_match: Option<PatternName>,
}

impl IsNodeConfig for ImageMatchNodeConfig {}

#[derive(Default)]
pub struct ImageMatchNode {
    /// Actual input image.
    input: Input<Image>,

    /// The image pattern against which is to be matched.
    config: ImageMatchNodeConfig,

    /// The actual pattern against which is being matched.
    pattern: Option<crate::pattern_match::Pattern>,

    /// The directory from which the patterns are loaded.
    directory: Option<std::path::PathBuf>,

    /// The available patterns for selection.
    pattern_library: Vec<PatternEntry>,
}
impl std::fmt::Debug for ImageMatchNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageMatchNode")
    }
}

impl ImageMatchNode {
    pub fn new() -> Self {
        ImageMatchNode::default()
    }

    fn load_patterns(&mut self) -> Result<(), NodeError> {
        if let Some(dir) = &self.directory {
            let mut dir = dir.clone();
            dir.push("image_match");
            self.pattern_library = load_patterns_directory(&dir)?;
        }
        Ok(())
    }
}

impl Node for ImageMatchNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.pattern.is_none() {
            let _ = self.load_patterns();
            if let Some(desired) = &self.config.use_match {
                if let Some(entry) = self
                    .pattern_library
                    .iter()
                    .find(|z| &z.info.name == desired)
                {
                    self.pattern = Some(entry.load_pattern()?);
                }
            }
        }
        if let Some(pattern) = &self.pattern {
            let image = self.input.get()?;
            // let start = std::time::Instant::now();
            if pattern.matches_exact(&image) {
                // println!("took: {:?}", std::time::Instant::now() - start);
                if ctx.children() == 0 {
                    return Ok(ExecutionStatus::Success);
                } else if ctx.children() == 1 {
                    return ctx.run(0);
                } else if ctx.children() > 1 {
                    return Err(format!("{:?} had more than one child", Self::static_type()).into());
                }
            } else {
                // println!("took: {:?}", std::time::Instant::now() - start);
                return Ok(ExecutionStatus::Failure);
            }
        }
        Err(format!(
            "no pattern or pattern not found: {:?}",
            self.config.use_match
        )
        .into())
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<Image>("image")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<Image>("image")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "image_match".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let before = self.config.use_match.clone();
        let r = self.config.load_node_config(config);
        if self.config.use_match != before {
            self.pattern = None;
        }
        r
    }

    fn set_directory(&mut self, directory: Option<&std::path::Path>) {
        self.directory = directory.map(|v| v.to_owned());
        let _ = self.load_patterns();
    }

    fn reset(&mut self) {
        self.pattern = None;
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    use betula_editor::{menu_node_recurser, UiMenuEntry, UiMenuNode, UiMenuTree};

    use crate::pattern_match::PatternInfo;
    impl UiMenuEntry for PatternInfo {
        fn label(&self) -> &str {
            self.name.0.as_ref()
        }
        fn hover(&self) -> Option<&str> {
            self.description.as_ref().map(|v| v.as_str())
        }
    }

    impl UiNode for ImageMatchNode {
        fn ui_title(&self) -> String {
            "image_match 🎇 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = (ctx, scale);

            /*

                struct UiMenuElement<T> {
                    leafs: Vec<(MenuInfo, T)>,
                    submenu: Vec<(MenuInfo, UiMenuElement<T>)>,
                }
            */

            let mut modified = false;

            ui.horizontal(|ui| {
                let label = if let Some(name) = self.config.use_match.clone() {
                    name.0
                } else {
                    "Select...".to_owned()
                };

                ui.menu_button(label, |ui| {
                    // Convert the pattern library to the menu tree.
                    type MenuType<'a> = UiMenuNode<PatternInfo, &'a PatternEntry>;
                    type TreeType<'a> = UiMenuTree<PatternInfo, &'a PatternEntry>;
                    let mut root = TreeType::new();
                    for pattern in self.pattern_library.iter() {
                        let h = pattern
                            .hierarchy
                            .clone()
                            .iter()
                            .map(|z| PatternInfo {
                                name: PatternName(z.clone()),
                                description: None,
                            })
                            .collect::<Vec<_>>();
                        let element = {
                            let mut element = &mut root;
                            for sub in h {
                                element = element
                                    .entry(sub)
                                    .or_insert_with(|| MenuType::SubElements(TreeType::new()))
                                    .sub_elements();
                            }
                            element
                        };
                        element.insert(pattern.info.clone(), MenuType::Value(pattern));
                    }
                    if let Some(entry) = menu_node_recurser(&root, ui) {
                        self.config.use_match = Some(entry.info.name.clone());
                        modified |= true;
                        ui.close_menu();
                    }
                });

                if ui
                    .button("🔃")
                    .on_hover_text("Reload patterns from directory.")
                    .clicked()
                {
                    let patterns = self.load_patterns();
                    if let Err(e) = patterns {
                        println!("Error loading patterns: {:?}", e)
                    }

                    ui.close_menu();
                }
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("image_match".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }
    }
}
