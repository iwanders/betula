use betula_overlay::client_server::{instructions::Text, OverlayClient, OverlayDaemonConfig};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1:5321")]
    bind: String,

    #[clap(long, short, action)]
    clear: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add text.
    Text(TextArgs),
    /// Remove all current elements.
    RemoveAllElements,
}

#[derive(Args)]
struct TextArgs {
    #[arg(short = 'p', value_name = "x,y", value_parser = parse_key_val)]
    pub position: (f32, f32),

    #[arg(short = 's', value_name = "w,h", value_parser = parse_key_val, default_value="100.0,100.0")]
    pub size: (f32, f32),

    #[arg(short = 'f', default_value = "10.0")]
    pub font_size: f32,

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

    if args.clear {
        client.remove_all_elements()?;
    }

    match args.command {
        Commands::Text(text_args) => {
            let text = Text {
                position: text_args.position,
                size: text_args.size,
                font_size: text_args.font_size,
                text: text_args.text,
                ..Default::default()
            };

            let _id = client.add_text(text)?;
            // std::thread::sleep(std::time::Duration::from_millis(1000));
            // client.remove(id)?;
        }
        Commands::RemoveAllElements => {
            client.remove_all_elements()?;
        }
    };

    // loop {
    //     std::thread::sleep(std::time::Duration::from_millis(1000));
    // }

    Ok(())
}
