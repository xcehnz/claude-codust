use anyhow::Result;
use clap::{Arg, Command};

mod config;
mod ui;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("claude-codust")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Claude Code configuration switcher")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Specify configuration file path to launch directly")
                .value_name("FILE")
                .action(clap::ArgAction::Set),
        )
        .get_matches();

    if let Some(config_path) = matches.get_one::<String>("config") {
        commands::launch_with_config_path(config_path).await?;
    } else {
        ui::show_interactive_selector().await?;
    }

    Ok(())
}