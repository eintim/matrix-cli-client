use matrix_sdk::{
    room::Room as MatrixRoom,
    ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
};
use tui::widgets::ListState;

use chrono::offset::Utc;
use chrono::DateTime;

#[derive(Debug, PartialEq, Eq)]
enum MessageViewMode {
    Follow,
    Scroll,
}

pub struct ScrollableMessageList {
    pub state: ListState,
    pub messages: Vec<(String, String, String)>,
    mode: MessageViewMode,
}

impl ScrollableMessageList {
    pub fn new() -> ScrollableMessageList {
        ScrollableMessageList {
            state: ListState::default(),
            messages: Vec::new(),
            mode: MessageViewMode::Follow,
        }
    }

    // pub fn with_messages(messages: Vec<(String, String)>) -> ScrollableMessageList {
    //     ScrollableMessageList {
    //         state: ListState::default(),
    //         messages: messages,
    //         mode: MessageViewMode::Follow,
    //     }
    // }

    pub fn add_message(&mut self, time: String, sender: String, message: String) {
        self.messages.push((time, sender, message));
        // Follow mode
        if self.mode == MessageViewMode::Follow {
            self.state.select(Some(self.messages.len() - 1));
        }
    }

    pub fn next_message(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    self.mode = MessageViewMode::Follow;
                    self.messages.len() - 1
                } else {
                    self.mode = MessageViewMode::Scroll;
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous_message(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    self.mode = MessageViewMode::Scroll;
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct Room {
    pub name: String,
    pub id: String,
    pub messages: ScrollableMessageList,
}

impl Room {
    pub async fn new(room: MatrixRoom) -> Room {
        let name = match room.display_name().await {
            Ok(name) => name.to_string(),
            Err(_) => "Unknown".to_string(),
        };

        //TODO Get past messages

        Room {
            name: name,
            id: room.room_id().to_string(),
            messages: ScrollableMessageList::new(),
        }
    }
}

pub struct ScrollableRoomList {
    pub state: ListState,
    pub rooms: Vec<Room>,
}

impl ScrollableRoomList {
    pub fn new() -> ScrollableRoomList {
        ScrollableRoomList {
            state: ListState::default(),
            rooms: Vec::new(),
        }
    }

    pub async fn add_room(&mut self, room: MatrixRoom) {
        let room = Room::new(room).await;
        self.rooms.push(room);
    }

    pub fn next_room(&mut self) {
        if self.rooms.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.rooms.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous_room(&mut self) {
        if self.rooms.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.rooms.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn get_current_room(&mut self) -> Option<&mut Room> {
        let i = match self.state.selected() {
            Some(i) => i,
            None => return None,
        };
        return self.rooms.get_mut(i);
    }
}

#[derive(PartialEq, Eq)]
pub enum Tabs {
    Room,
    //Members,
    Messages,
    //Input,
}

pub struct App {
    pub rooms: ScrollableRoomList,
    pub logged_in: bool,
    pub current_tab: Tabs,
}

impl Default for App {
    fn default() -> App {
        App {
            rooms: ScrollableRoomList::new(),
            logged_in: false,
            current_tab: Tabs::Room,
        }
    }
}

impl App {
    pub fn new() -> App {
        App::default()
    }
    pub fn handle_matrix_event(
        &mut self,
        event: OriginalSyncRoomMessageEvent,
        room: matrix_sdk::room::Room,
    ) {
        let room = room.room_id().to_string();
        let system_time = match event.origin_server_ts.to_system_time() {
            Some(time) => time,
            None => return,
        };
        let datetime: DateTime<Utc> = system_time.into();

        let sender = event.sender.to_string();
        let message = event.content;

        match self.rooms.rooms.iter_mut().find(|r| r.id == room) {
            Some(r) => {
                r.messages.add_message(
                    datetime.format("%d/%m/%Y %T").to_string(),
                    sender,
                    convert_message_type(message.msgtype),
                );
            }
            None => {}
        }
    }
    pub fn current_room_next_message(&mut self) {
        if let Some(r) = self.rooms.get_current_room() {
            r.messages.next_message();
        }
    }
    pub fn current_room_previous_message(&mut self) {
        if let Some(r) = self.rooms.get_current_room() {
            r.messages.previous_message();
        }
    }

    pub fn next_tab(&mut self) {
        match self.current_tab {
            Tabs::Room => self.current_tab = Tabs::Messages,
            Tabs::Messages => self.current_tab = Tabs::Room,
            //_ => self.current_tab = Tabs::Room,
        }
    }
}

fn convert_message_type(msgtype: MessageType) -> String {
    match msgtype {
        MessageType::Text(content) => content.body,
        MessageType::Audio(content) => "Has send audio: ".to_string() + &content.body,
        //MessageType::Emote(content) => "Has send Sticker: ".to_string() + &content.body,
        MessageType::File(content) => "Has send file: ".to_string() + &content.body,
        MessageType::Image(content) => "Has send image: ".to_string() + &content.body,
        MessageType::Video(content) => "Has send video: ".to_string() + &content.body,
        MessageType::Location(content) => "Has send location: ".to_string() + &content.geo_uri,
        _ => "Unknown messagetype".to_string(),
    }
}
