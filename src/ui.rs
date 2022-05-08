use crate::app::{App, Tabs};

use crossterm::event::{self, poll, Event, KeyCode};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

use matrix_sdk::{
    config::SyncSettings, room::Room, ruma::events::room::message::OriginalSyncRoomMessageEvent,
    Client,
};

use url::Url;

pub async fn run_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    username: String,
    password: String,
    homeserver: Url,
) -> io::Result<()> {
    // let (send_tx, send_rx) = mpsc::channel(100);
    let (recv_tx, mut recv_rx) = mpsc::channel(100);

    let client = match Client::new(homeserver).await {
        Ok(client) => client,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not create client",
            ));
        }
    };

    match client
        .login(&username, &password, None, Some("Matrix-Tui-Client"))
        .await
    {
        Ok(_) => (),
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not login. Invalid password?",
            ));
        }
    };

    //Event Handler
    client
        .register_event_handler({
            let tx = recv_tx.clone();
            move |ev: OriginalSyncRoomMessageEvent, room: Room| {
                let tx = tx.clone();
                async move {
                    match tx.send((ev, room)).await {
                        Ok(_) => (),
                        Err(_) => (),
                    };
                }
            }
        })
        .await;

    match client.sync_once(SyncSettings::default()).await {
        Ok(_) => (),
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unable to sync with homeserver",
            ));
        }
    };

    //Get all rooms
    let rooms = client.rooms();
    for room in rooms {
        app.rooms.add_room(room).await;
    }

    let client2 = client.clone();
    tokio::spawn(async move {
        client2.sync(SyncSettings::default()).await;
    });

    loop {
        // Check rx
        if let Some((ev, room)) = recv_rx.try_recv().ok() {
            app.handle_matrix_event(ev, room);
        }

        terminal.draw(|f| ui(f, &mut app))?;

        if poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match app.current_tab {
                    Tabs::Room => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            return Ok(());
                        }
                        KeyCode::Up => {
                            app.rooms.previous_room();
                        }
                        KeyCode::Down => {
                            app.rooms.next_room();
                        }
                        KeyCode::Tab => {
                            app.next_tab();
                        }
                        _ => {}
                    },
                    Tabs::Messages => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            return Ok(());
                        }
                        KeyCode::Up => app.current_room_previous_message(),
                        KeyCode::Down => app.current_room_next_message(),
                        KeyCode::Tab => {
                            app.next_tab();
                        }
                        _ => {}
                    },
                    //_ => {}
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(85)].as_ref())
        .split(f.size());

    //Room Select Widget
    let rooms: Vec<ListItem> = app
        .rooms
        .rooms
        .iter()
        .enumerate()
        .map(|(_i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}", m.name)))];
            ListItem::new(content)
        })
        .collect();
    let block_rooms = match app.current_tab {
        Tabs::Room => Block::default()
            .borders(Borders::ALL)
            .title("Rooms")
            .border_type(BorderType::Thick),
        _ => Block::default().borders(Borders::ALL).title("Rooms"),
    };

    let rooms = List::new(rooms)
        .block(block_rooms)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    f.render_stateful_widget(rooms, chunks[0], &mut app.rooms.state);

    // Message Widget
    match app.rooms.get_current_room() {
        Some(room) => {
            let messages: Vec<ListItem> = room
                .messages
                .messages
                .iter()
                .enumerate()
                .map(|(_i, m)| {
                    let content = vec![Spans::from(vec![
                        Span::styled(format!("{}", m.0), Style::default().fg(Color::Green)),
                        Span::styled(format!("{}", m.1), Style::default().fg(Color::Red)),
                        Span::from(format!("{}", m.2)),
                    ])];
                    ListItem::new(content)
                })
                .collect();

            let block_message = match app.current_tab {
                Tabs::Messages => Block::default()
                    .borders(Borders::ALL)
                    .title("Messages")
                    .border_type(BorderType::Thick),
                _ => Block::default().borders(Borders::ALL).title("Messages"),
            };
            let messages = List::new(messages)
                .block(block_message)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            f.render_stateful_widget(messages, chunks[1], &mut room.messages.state);
        }
        None => {
            let text = vec![
                Spans::from("This is a Matrix Tui Client"),
                Spans::from(""),
                Spans::from("To switch between tabs use tab key"),
                Spans::from("To scroll up and down use up and down arrow keys"),
                Spans::from("To quit the client use q"),
            ];
            let block = match app.current_tab {
                Tabs::Messages => Block::default()
                    .borders(Borders::ALL)
                    .title("Welcome")
                    .border_type(BorderType::Thick),
                _ => Block::default().borders(Borders::ALL).title("Welcome"),
            };
            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
            f.render_widget(paragraph, chunks[1]);
        }
    };
}
