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

    #[derive(Clone, Hash, Debug, Ord, Eq, PartialEq, PartialOrd)]
    struct MenuInfo {
        label: String,
        hover: Option<String>,
    }

    type UiMenuTree<T> = std::collections::BTreeMap<MenuInfo, UiMenuNode<T>>;
    enum UiMenuNode<T> {
        Value(T),
        SubElements(UiMenuTree<T>),
    }
    impl<T> UiMenuNode<T> {
        pub fn sub_elements(&mut self) -> &mut UiMenuTree<T> {
            if let UiMenuNode::<T>::SubElements(z) = self {
                return z;
            }
            panic!("sub elements called on non subelement enum");
        }
    }

    fn menu_node_recurser<T: Copy>(tree: &UiMenuTree<T>, ui: &mut egui::Ui) -> Option<T> {
        for (info, element) in tree.iter() {
            match element {
                UiMenuNode::<T>::Value(ref v) => {
                    let mut button = ui.button(info.label.clone());
                    if let Some(s) = info.hover.as_ref() {
                        button = button.on_hover_text(s);
                    }
                    if button.clicked() {
                        ui.close_menu();
                        return Some(*v);
                    }
                }
                UiMenuNode::<T>::SubElements(ref subtree) => {
                    let z =
                        ui.menu_button(info.label.clone(), |ui| menu_node_recurser(subtree, ui));
                    if let Some(returned_node_type) = z.inner.flatten() {
                        return Some(returned_node_type);
                    }
                }
            }
        }

        None
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

                // Convert the pattern library to the menu tree.
                type MenuType<'a> = UiMenuNode<&'a PatternEntry>;
                type TreeType<'a> = UiMenuTree<&'a PatternEntry>;
                let mut root = TreeType::new();
                for pattern in self.pattern_library.iter() {
                    let h = pattern
                        .hierarchy
                        .clone()
                        .iter()
                        .map(|z| MenuInfo {
                            label: z.clone(),
                            hover: None,
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
                    element.insert(
                        MenuInfo {
                            label: pattern.info.name.0.clone(),
                            hover: pattern.info.description.clone(),
                        },
                        MenuType::Value(pattern),
                    );
                }
                ui.menu_button(label, |ui| {
                    if let Some(entry) = menu_node_recurser(&root, ui) {
                        self.config.use_match = Some(entry.info.name.clone());
                        modified |= true;
                        ui.close_menu();
                    }
                });
                /*
                ui.menu_button(label, |ui| {
                    for entry in self.pattern_library.iter() {
                        let mut button = ui.button(entry.info.name.0.clone());
                        if let Some(description) = entry.info.description.as_ref() {
                            button = button.on_hover_text(description);
                        }
                        if button.clicked() {
                            self.config.use_match = Some(entry.info.name.clone());
                            modified |= true;
                            ui.close_menu();
                        }
                    }
                });
                */

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
