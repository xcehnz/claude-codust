use anyhow::Result;
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

use crate::config::{ConfigItem};
use crate::commands::switch_configuration;

pub async fn show_interactive_selector() -> Result<()> {
    let configs = crate::config::load_configurations()?;
    
    if configs.is_empty() {
        println!("No configuration files found in ~/.claude/ or ~/.claude-code-router/");
        return Ok(());
    }

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;

    let result = run_selector(&configs).await;

    execute!(io::stdout(), crossterm::cursor::Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    result
}

async fn run_selector(configs: &[ConfigItem]) -> Result<()> {
    let mut selected = 0;

    loop {
        print_selector_ui(configs, selected)?;

        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event::read()?
        {
            match code {
                KeyCode::Up => {
                    if selected == 0 {
                        selected = configs.len() - 1;
                    } else {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected == configs.len() - 1 {
                        selected = 0;
                    } else {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    switch_configuration(&configs[selected]).await?;
                    return Ok(());
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    println!("\r\nCancelled");
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

fn print_selector_ui(configs: &[ConfigItem], selected: usize) -> Result<()> {
    execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0))?;
    execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::FromCursorDown))?;

    println!(" Claude Code Configuration Selector");
    println!(" Use ↑/↓ to navigate, Enter to select, Esc/q to quit");
    println!();

    let max_name_len = configs.iter()
        .map(|c| c.name.len() + c.config_type.get_indicator().len())
        .max()
        .unwrap_or(0);

    for (i, config) in configs.iter().enumerate() {
        let prefix = if i == selected { "❯ " } else { "  " };
        let type_indicator = config.config_type.get_indicator();
        let name_with_indicator = format!("{}{}", config.name, type_indicator);
        println!("{}{:<width$} {}", prefix, name_with_indicator, config.path.display(), width = max_name_len);
    }

    io::stdout().flush()?;
    Ok(())
}