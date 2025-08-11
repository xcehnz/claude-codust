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
            Arg::new("code")
                .help("Show interactive configuration selector")
                .action(clap::ArgAction::Set)
                .required(false),
        )
        .get_matches();

    if matches.contains_id("code") {
        ui::show_interactive_selector().await?;
    } else {
        println!("Use 'code' to show interactive configuration selector");
    }

    Ok(())
}