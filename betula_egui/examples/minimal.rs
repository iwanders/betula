use eframe::{App, CreationContext};
use egui::{Color32, Ui};
use egui_snarl::{
    ui::{PinInfo, SnarlStyle, SnarlViewer},
    InPin, NodeId, OutPin, Snarl,
};

const STRING_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0x00);
const UNTYPED_COLOR: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb0);
const RELATION_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0xb0);

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    String(String),

    /// Tree element with string input and variable horizontal outputs.
    Tree(usize /* used outputs */, String /* current value */),
}

impl DemoNode {}

struct DemoViewer;

fn handle_tree_outputs(node: NodeId, snarl: &mut Snarl<DemoNode>) {
    let current_count;
    if let DemoNode::Tree(_, _) = snarl[node] {
        current_count = snarl.out_pins_connected(node).map(|v| v.output).max();
    } else {
        return;
    }

    let mut values_in_use = 0;
    if let Some(v) = current_count {
        // truncate to the first output that has remotes.
        for i in 0..=v{
            let outpinid = egui_snarl::OutPinId{node, output: i};
            let full_pin = snarl.out_pin(outpinid);
            if !full_pin.remotes.is_empty(){
                values_in_use = i + 1;  // +1 to go from index to count.
            }
        }
    }

    if let DemoNode::Tree(ref mut v, _) = snarl[node]{
        *v = values_in_use;
    }
}

impl SnarlViewer<DemoNode> for DemoViewer {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DemoNode>) {
        // Validate connection
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (DemoNode::Sink, _) => {
                unreachable!("Sink node has no outputs")
            }
            (DemoNode::Tree(_, _), _) => {
            }
            (_, DemoNode::Sink) => {}
            (_, DemoNode::String(_)) => {
                unreachable!("String node has no inputs")
            }
            (_, _) => { }
        }

        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl.connect(from.id, to.id);

        handle_tree_outputs(from.id.node, snarl);
        handle_tree_outputs(to.id.node, snarl);
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DemoNode>) {
        snarl.disconnect(from.id, to.id);
        handle_tree_outputs(from.id.node, snarl);
        handle_tree_outputs(to.id.node, snarl);
    }

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<DemoNode>) {
        snarl.drop_outputs(pin.id);
        handle_tree_outputs(pin.id.node, snarl);
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<DemoNode>) {
        snarl.drop_inputs(pin.id);
        handle_tree_outputs(pin.id.node, snarl);
    }

    fn title(&mut self, node: &DemoNode) -> String {
        match node {
            DemoNode::Sink => "Sink".to_owned(),
            DemoNode::String(_) => "String".to_owned(),
            DemoNode::Tree(_, _) => "Tree".to_owned(),
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 1,
            DemoNode::String(_) => 0,
            DemoNode::Tree(_, _) => 2, // string input and parent input.
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::String(_) => 1,
            DemoNode::Tree(children, _) => children + 1, // children 
        }
    }

    fn vertical_input(
        &mut self,
        pin: &InPin,
        snarl: &mut Snarl<DemoNode>
    ) -> Option<PinInfo> {
        match snarl[pin.id.node] {
            DemoNode::Tree(_, _) => {
                if pin.id.input == 0 {
                    Some(PinInfo::triangle().with_fill(RELATION_COLOR).vertical())
                } else {
                    None
                }
            }
            _ => None
        }
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");

                match &*pin.remotes {
                    [] => {
                        ui.label("None");
                        PinInfo::circle().with_fill(UNTYPED_COLOR)
                    }
                    [remote] => match snarl[remote.node] {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Tree(_, ref value) => {
                            // assert_eq!(remote.output, 0, "Number node has only one output");
                            // ui.label(format_float(value));
                            ui.label(format!("{}:{}", value, remote.output));
                            PinInfo::square().with_fill(RELATION_COLOR)
                        }
                        DemoNode::String(ref value) => {
                            assert_eq!(remote.output, 0, "String node has only one output");
                            ui.label(format!("{}", value));
                            PinInfo::triangle().with_fill(STRING_COLOR)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Tree(_, _) => {
                // Just collect both inputs to update the interior string here.
                let root_pin = snarl.in_pin(egui_snarl::InPinId{node: pin.id.node, input: 0});
                let input_pin = snarl.in_pin(egui_snarl::InPinId{node: pin.id.node, input: 1});
                let mut root_string = "".to_owned();
                let mut input_string = "".to_owned();
                for pin in [root_pin, input_pin]{
                    for remote in pin.remotes.iter() {
                        let dest = if pin.id.input == 0 { &mut root_string} else {&mut input_string};
                        match snarl[remote.node] {
                            DemoNode::Sink => unreachable!("Sink node has no outputs"),
                            DemoNode::Tree(_, ref root_string_value) => {
                                *dest = root_string_value.to_owned() + format!(":{}", remote.output).as_str();
                            }
                            DemoNode::String(ref value) => {
                                *dest = value.to_owned();
                            }
                        }
                    }
                }
                {
                    let node = &mut snarl[pin.id.node];
                    if let DemoNode::Tree(_, ref mut v) = node {
                        *v = format!("{input_string}{}{root_string}", if !input_string.is_empty() {"_"} else {""});
                    }
                }
                if pin.id.input == 0 {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                } else {
                    ui.label(format!("Input {input_string}"));
                    PinInfo::triangle().with_fill(STRING_COLOR)
                }
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::String(ref mut value) => {
                assert_eq!(pin.id.output, 0, "String node has only one output");
                let edit = egui::TextEdit::singleline(value)
                    .clip_text(false)
                    .desired_width(0.0)
                    .margin(ui.spacing().item_spacing);
                ui.add(edit);
                PinInfo::triangle().with_fill(STRING_COLOR)
            }
            DemoNode::Tree(_, _) => {
                // You could draw elements here, like a label:
                // ui.add(egui::Label::new(format!("{:?}", pin.id.output)));
                if pin.remotes.is_empty() {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical().wiring().with_gamma(0.5)
                } else {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                }
            }
        }
    }

    fn vertical_output(
        &mut self,
        pin: &OutPin,
        snarl: &mut Snarl<DemoNode>
    ) -> Option<PinInfo> {
        match snarl[pin.id.node] {
            DemoNode::Tree(_, _) => {
                if pin.remotes.is_empty() {
                    Some(PinInfo::triangle().with_fill(RELATION_COLOR).vertical().wiring().with_gamma(0.5))
                } else {
                    Some(PinInfo::triangle().with_fill(RELATION_COLOR).vertical())
                }
            }
            _ => None
        }
    }

    fn input_color(
        &mut self,
        pin: &InPin,
        _style: &egui::Style,
        snarl: &mut Snarl<DemoNode>,
    ) -> Color32 {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");
                match &*pin.remotes {
                    [] => UNTYPED_COLOR,
                    [remote] => match snarl[remote.node] {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::String(_) => STRING_COLOR,
                        DemoNode::Tree(_, _) => RELATION_COLOR,
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
            DemoNode::Tree(_, _) => {
                RELATION_COLOR
            }
        }
    }

    fn output_color(
        &mut self,
        pin: &OutPin,
        _style: &egui::Style,
        snarl: &mut Snarl<DemoNode>,
    ) -> Color32 {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::String(_) => STRING_COLOR,
            DemoNode::Tree(_, _) => RELATION_COLOR,
        }
    }

    fn graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Add node");
        if ui.button("String").clicked() {
            snarl.insert_node(pos, DemoNode::String("".to_owned()));
            ui.close_menu();
        }
        if ui.button("Sink").clicked() {
            snarl.insert_node(pos, DemoNode::Sink);
            ui.close_menu();
        }
        if ui.button("Tree").clicked() {
            snarl.insert_node(pos, DemoNode::Tree(0, "".to_owned()));
            ui.close_menu();
        }
    }

    fn node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            snarl.remove_node(node);
            ui.close_menu();
        }
    }

    fn has_on_hover_popup(&mut self, _: &DemoNode) -> bool {
        true
    }

    fn show_on_hover_popup(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        match snarl[node] {
            DemoNode::Sink => {
                ui.label("Displays anything connected to it");
            }
            DemoNode::String(_) => {
                ui.label("Outputs string value");
            }
            DemoNode::Tree(_, _) => {
                ui.label("Can have relations");
            }
        }
    }

    fn has_footer(&mut self, _v: &DemoNode) -> bool {
        true
    }

    fn show_footer(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        match &snarl[node] {
            DemoNode::Tree(c, v) => {
                ui.label(format!("{c} outputs"));
                ui.label(format!("value: {v}"));
            }
            _ => ()
        }
    }
    
}

pub struct DemoApp {
    snarl: Snarl<DemoNode>,
    style: SnarlStyle,
}

impl DemoApp {
    pub fn new(cx: &CreationContext) -> Self {
        let snarl = Snarl::<DemoNode>::new();
        // let snarl = Snarl::<DemoNode>::new();

        let style = SnarlStyle::new();
        // let style = SnarlStyle::new();

        DemoApp { snarl, style }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {


        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });


        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl
                .show(&mut DemoViewer, &self.style, egui::Id::new("snarl"), ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // let snarl = serde_json::to_string(&self.snarl).unwrap();
        // storage.set_string("snarl", snarl);

        // let style = serde_json::to_string(&self.style).unwrap();
        // storage.set_string("style", style);
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|cx| Box::new(DemoApp::new(cx))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "egui_snarl_demo",
                web_options,
                Box::new(|cx| Box::new(DemoApp::new(cx))),
            )
            .await
            .expect("failed to start eframe");
    });
}
