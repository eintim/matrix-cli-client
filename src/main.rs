mod app;
mod matrix;
mod ui;

use clap::Parser;

use crate::app::App;
use crate::matrix::*;
use crate::ui::run_ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tokio::sync::mpsc;

use matrix_sdk::Client;

use std::io;
use tui::{backend::CrosstermBackend, Terminal};
use url::Url;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Matrix Homeserver
    #[clap(default_value = "https://matrix.org")]
    home_server: String,

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

    let homeserver_url = match Url::parse(&args.home_server) {
        Ok(url) => url,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not parse homeserver url",
            ));
        }
    };

    // initialize channel
    let (tx, rx) = mpsc::channel(100);

    // initialize matrix client
    let client = match Client::initialize(homeserver_url, args.username, args.password, tx).await {
        Ok(client) => client,
        Err(err) => {
            return Err(io::Error::new(io::ErrorKind::Other, err.to_string()));
        }
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run ui
    let app = App::new(client).await;
    let res = run_ui(&mut terminal, app, rx).await;

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
