use betula_overlay::client_server::{OverlayDaemonConfig, OverlayServer};
use screen_overlay::{Overlay, OverlayConfig, OverlayHandle};

use clap::Parser;

/// A program capable of drawing a fullscreen image as an overlay.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Width of the overlay.
    #[arg(short, long, default_value_t = 1920.0)]
    width: f32,

    /// Height of the overlay.
    #[arg(short, long, default_value_t = 1080.0)]
    height: f32,

    /// x position of the overlay.
    #[arg(short, long, default_value_t = 0.0)]
    x: f32,
    /// y position of the overlay.
    #[arg(short, long, default_value_t = 0.0)]
    y: f32,

    /// Use debug fill for the overlay.
    #[arg(short, long, default_value_t = false)]
    debug_fill: bool,

    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1:5321")]
    bind: String,
}

struct OverlayDaemon {
    overlay: OverlayHandle,
    server: OverlayServer,
}

impl eframe::App for OverlayDaemon {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = (ctx, frame);
        let z = self.server.service();
        if let Err(e) = z {
            println!("Something went wrong: {e:?}");
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.overlay.configure(ui);
        self.overlay.draw(ui);
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}

pub fn main() -> std::result::Result<(), betula_overlay::OverlayError> {
    let args = Args::parse();
    let config = OverlayConfig::new()
        .with_size([args.width, args.height])
        .with_position([args.x, args.y])
        .with_central_panel_fill(if args.debug_fill {
            screen_overlay::DEBUG_COLOR
        } else {
            screen_overlay::egui::Color32::TRANSPARENT
        });
    let overlay = Overlay::new(config);
    let handle = OverlayHandle::new(overlay);
    let overlay = handle.clone();
    let overlay2 = handle.clone();
    let config = OverlayDaemonConfig {
        bind: args.bind.parse()?,
    };
    let server = OverlayServer::new(config, handle)?;
    let daemon = OverlayDaemon { overlay, server };

    eframe::run_native(
        "Image Overlay",
        overlay2.native_options(),
        Box::new(|cc| {
            let _ = cc;
            // This gives us image support:
            // egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(daemon))
        }),
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
