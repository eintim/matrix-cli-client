use clap::Parser;

use matrix_sdk::{
    ruma::events::{room::message::MessageEventContent, SyncMessageEvent},
    Client, Result, SyncSettings,
};
use url::Url;

use tokio::sync::mpsc;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};

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

struct App {
    /// History of recorded messages
    messages: Vec<(String, String)>,
}

impl Default for App {
    fn default() -> App {
        App {
            messages: Vec::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel(100);

    let args = Args::parse();

    let homeserverurl = Url::parse(&args.home_server)?;

    let client = Client::new(homeserverurl)?;
    // First we need to log in.
    client
        .login(&args.username, &args.password, None, None)
        .await?;

    client
        .register_event_handler({
            let tx = tx.clone();
            move |ev: SyncMessageEvent<MessageEventContent>| {
                let tx = tx.clone();
                async move {
                    tx.send(ev).await.unwrap();
                }
            }
        })
        .await;

    // Syncing is important to synchronize the client state with the server.
    // This method will never return.
    tokio::spawn(async move {
        client.sync(SyncSettings::default()).await;
    });

    //UI Stuff

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let _res = run_app(&mut terminal, app, rx);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // create app and run it

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut rx: mpsc::Receiver<SyncMessageEvent<MessageEventContent>>,
) -> io::Result<()> {
    loop {
        //Check rx
        if let Some(ev) = rx.try_recv().ok() {
            app.messages.push((
                ev.sender.to_string(),
                (format!("{:?}", ev.content.msgtype)).to_string(),
            ));
        }

        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(f.size());
    let block = Block::default()
        .title("Matrix-Client")
        .borders(Borders::ALL);
    f.render_widget(block, chunks[0]);

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(_i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", m.0, m.1)))];
            ListItem::new(content)
        })
        .collect();
    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(messages, chunks[1]);
}
