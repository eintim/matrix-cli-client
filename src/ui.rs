use crate::app::App;

use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};

use matrix_sdk::{
    ruma::{
        events::{room::message::MessageEventContent, SyncMessageEvent},
        RoomId, UserId,
    },
    Client, Result, SyncSettings,
};

pub async fn run_tui(username: String, password: String) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let _res = run_app(&mut terminal, app, username, password).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    username: String,
    password: String,
) -> io::Result<()> {
    // let (send_tx, send_rx) = mpsc::channel(100);
    let (recv_tx, mut recv_rx) = mpsc::channel(100);

    let user_id = UserId::try_from(username.clone()).unwrap();
    let client = Client::new_from_user_id(user_id.clone()).await.unwrap();

    client
        .login(&username, &password, None, Some("Matrix-Tui-Client"))
        .await
        .unwrap();

    //Event Handler
    client
        .register_event_handler({
            let tx = recv_tx.clone();
            move |ev: SyncMessageEvent<MessageEventContent>| {
                let tx = tx.clone();
                async move {
                    tx.send(ev).await.unwrap();
                }
            }
        })
        .await;

    client.sync_once(SyncSettings::default()).await.unwrap();
    let client2 = client.clone();
    tokio::spawn(async move {
        client2.sync(SyncSettings::default()).await;
    });

    loop {
        // Check rx
        if let Some(ev) = recv_rx.try_recv().ok() {
            app.messages.push((
                ev.sender.to_string(),
                (format!("{:?}", ev.content.msgtype)).to_string(),
            ));
        }

        terminal.draw(|f| ui(f, &app))?;

        if poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Enter => {
                        let room_id =
                            RoomId::try_from("!EMdVAVFRONBxBEGesT:jannikspringer.de").unwrap();
                        let room = client.get_joined_room(&room_id).unwrap();
                        let content =
                            MessageEventContent::text_plain(format!("🎉🎊🥳 let's PARTY!! 🥳🎊🎉"));
                        room.send(content, None).await.unwrap();
                    }
                    _ => {}
                }
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
