use betula_common::{control::internal_server_client, create_server_thread};
use betula_core::basic::{BasicBlackboard, BasicTree};
use betula_editor::{editor::BetulaEditor, UiSupport};

// Factory function for the ui support.
fn create_ui_support() -> UiSupport {
    let mut ui_support = UiSupport::new();
    ui_support
        .tree_support_mut()
        .set_blackboard_factory(Box::new(|| {
            Box::new(betula_core::basic::BasicBlackboard::default())
        }));
    betula_enigo::add_ui_support(&mut ui_support);
    betula_std::add_ui_support(&mut ui_support);
    betula_wm::add_ui_support(&mut ui_support);
    betula_hotkey::add_ui_support(&mut ui_support);
    betula_image::add_ui_support(&mut ui_support);
    betula_overlay::add_ui_support(&mut ui_support);
    ui_support
}

fn service_overlays(editor: &mut BetulaEditor, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    let _ = (editor, frame);
    let overlays = betula_overlay::get_overlays();
    for v in overlays {
        // println!("drawing {:?}", std::sync::Weak::as_ptr(&v.to_weak()));
        v.show_viewport_deferred(ui);
    }
}

fn main() -> eframe::Result<()> {
    // Create the control pipes.
    let (server, client) = internal_server_client();

    // Create the background runner.
    let _background_runner = create_server_thread::<BasicTree, BasicBlackboard>(
        Box::new(|| create_ui_support().into_tree_support()),
        server,
    );

    // Create the viewer
    let ui_support = create_ui_support();


    #[cfg(target_os = "windows")]
    let renderer = eframe::Renderer::Glow;
    #[cfg(not(target_os = "windows"))]
    let renderer = eframe::Renderer::Wgpu;

    // Run the editor.
    let mut native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_transparent(true),
        // multisampling: 1,
        renderer,
        ..Default::default()
    };
    native_options.viewport.icon = Some(std::sync::Arc::new(betula_editor::betula_icon()));

    // Populate the options.
    let mut options = betula_editor::editor::EditorOptions::default();
    let args: Vec<String> = std::env::args().collect();
    if let Some(fpath) = args.get(1) {
        if fpath == "--help" {
            eprintln!("./betula_demo path_to_tree.json");
            std::process::exit(1);
        }
        let path = std::path::PathBuf::from(fpath);
        if path.is_file() {
            options.open_file = Some(path);
        } else {
            eprintln!("File path to {fpath} did not exist, or unknown argument");
            std::process::exit(1);
        }
    }

    eframe::run_native(
        "Betula Interface",
        native_options,
        Box::new(move |cx| {
            let mut editor = BetulaEditor::new(Box::new(client), ui_support, cx, &options);
            editor.add_ui_callback(Box::new(service_overlays));
            Ok(Box::new(editor))
        }),
    )
}
