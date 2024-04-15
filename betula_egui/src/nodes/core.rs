use crate::{ui::UiNodeCategory, UiNode};
use betula_core::nodes;

impl UiNode for nodes::SequenceNode {
    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("logic".to_owned()),
            UiNodeCategory::Group("core".to_owned()),
            UiNodeCategory::Name("sequence".to_owned()),
        ]
    }
}

impl UiNode for nodes::SuccessNode {
    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("decorator".to_owned()),
            UiNodeCategory::Group("core".to_owned()),
            UiNodeCategory::Name("success".to_owned()),
        ]
    }
}

impl UiNode for nodes::SelectorNode {
    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("logic".to_owned()),
            UiNodeCategory::Group("core".to_owned()),
            UiNodeCategory::Name("selector".to_owned()),
        ]
    }
}

impl UiNode for nodes::FailureNode {
    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("decorator".to_owned()),
            UiNodeCategory::Group("core".to_owned()),
            UiNodeCategory::Name("failure".to_owned()),
        ]
    }
}
