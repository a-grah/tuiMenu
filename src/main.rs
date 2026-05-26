mod app;
mod config;
mod ui;
mod widgets;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{run_command_blocking, App};
use config::Config;
use widgets::Widgets;

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    if std::env::args().skip(1).any(|a| a == "--check") {
        return run_check();
    }

    let config = Config::load()?;
    let mut app = App::new(config);
    let mut widgets = Widgets::new(&app.config.settings);

    let mut terminal = setup_terminal()?;
    let res = run_loop(&mut terminal, &mut app, &mut widgets);
    restore_terminal(&mut terminal)?;
    res
}

/// Non-interactive smoke test: validate config and collect every widget once.
fn run_check() -> Result<()> {
    let config = Config::load()?;
    println!(
        "config: {} entries at {}",
        config.entries.len(),
        config::config_path().display()
    );

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    println!("hostname: {}", widgets::short_hostname());
    println!("{}", widgets::memory_string(&sys));
    match widgets::fetch_btc(&config.settings) {
        Some(s) => println!("{s}"),
        None => println!("BTC: (fetch failed — offline?)"),
    }
    match widgets::fetch_weather(&config.settings) {
        Some(s) => println!("weather: {s}"),
        None => println!("weather: (fetch failed — offline?)"),
    }
    Ok(())
}

fn run_loop(terminal: &mut Tui, app: &mut App, widgets: &mut Widgets) -> Result<()> {
    loop {
        widgets.tick();
        terminal.draw(|f| ui::draw(f, app, widgets))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key);
                }
            }
        }

        if app.should_quit {
            break;
        }

        if let Some(cmd) = app.run_request.take() {
            restore_terminal(terminal)?;
            run_command_blocking(&cmd)?;
            *terminal = setup_terminal()?;
            terminal.clear()?;
        }
    }
    Ok(())
}

fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
