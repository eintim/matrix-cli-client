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
mod app;

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = app::App::default();
    let _res = run_app(&mut terminal, app, rx);
    // start matrix && register event-handler


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

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: app::App,
    mut rx: mpsc::Receiver<SyncMessageEvent<MessageEventContent>>,
) -> io::Result<()> {
    if !(app.logged_in) {
        // login
        // start matrix client
    }
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

fn ui<B: Backend>(f: &mut Frame<B>, app: &app::App) {
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
