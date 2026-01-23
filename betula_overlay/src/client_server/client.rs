use betula_overlay::client_server::{instructions::Text, OverlayClient, OverlayDaemonConfig};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1:5321")]
    bind: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Text(TextArgs),
    RemoveAllElements,
}

#[derive(Args)]
struct TextArgs {
    #[arg(short = 'p', value_name = "x,y", value_parser = parse_key_val)]
    pub position: (f32, f32),
    // pub size: (f32, f32),

    // pub font_size: f32,

    // pub text_color: Color32,

    // pub fill_color: Color32,
    pub text: String,
}

/// Parse a single key-value pair
fn parse_key_val(
    s: &str,
) -> Result<(f32, f32), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let pos = s.find(',').ok_or_else(|| format!("no , found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

pub fn main() -> std::result::Result<(), betula_overlay::OverlayError> {
    let args = Cli::parse();

    let config = OverlayDaemonConfig {
        bind: args.bind.parse()?,
    };
    let client = OverlayClient::new(config);

    match args.command {
        Commands::Text(text_args) => {
            let text = Text {
                position: text_args.position,
                text: text_args.text,
                ..Default::default()
            };

            client.add_text(text)?;
        }
        Commands::RemoveAllElements => {
            client.remove_all_elements()?;
        }
    };

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    Ok(())
}
