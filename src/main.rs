use clap::Parser;

use crate::app::App;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod ui;
use crate::ui::run_ui;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Username
    #[clap(short)]
    username: String,

    /// Password
    #[clap(short)]
    password: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // parse args
    let args = Args::parse();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run ui
    let app = App::new();
    let res = run_ui(&mut terminal, app, args.username, args.password).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // return result of ui
    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
