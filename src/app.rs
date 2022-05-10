use crate::matrix::convert_message_type;

use matrix_sdk::{
    room::Room as MatrixRoom, ruma::events::room::message::OriginalSyncRoomMessageEvent,
};
use tui::widgets::ListState;

use chrono::offset::Utc;
use chrono::DateTime;

use url::Url;

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

pub struct ScrollableMemberList {
    pub state: ListState,
    pub members: Vec<String>,
}

impl ScrollableMemberList {
    pub fn with_members(members: Vec<String>) -> ScrollableMemberList {
        ScrollableMemberList {
            state: ListState::default(),
            members: members,
        }
    }

    pub fn next_member(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.members.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous_member(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.members.len() - 1
                } else {
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
    pub members: ScrollableMemberList,
}

impl Room {
    pub async fn new(room: MatrixRoom) -> Room {
        let name = match room.display_name().await {
            Ok(name) => name.to_string(),
            Err(_) => "Unknown".to_string(),
        };

        let members = match room.joined_members().await {
            Ok(members) => members,
            Err(_) => Vec::new(),
        };

        let member_names = members
            .into_iter()
            .map(|member| match member.display_name() {
                Some(name) => name.to_string(),
                None => member.user_id().to_string(),
            })
            .collect::<Vec<String>>();

        Room {
            name: name,
            id: room.room_id().to_string(),
            messages: ScrollableMessageList::new(),
            members: ScrollableMemberList::with_members(member_names),
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
    Members,
    Messages,
    Input,
}

pub struct App {
    pub rooms: ScrollableRoomList,
    pub current_tab: Tabs,
    pub input: String,
    homeserver_url: Url,
    full_username: String,
}

impl App {
    pub fn new(homeserver_url: Url, full_username: String) -> App {
        App {
            rooms: ScrollableRoomList::new(),
            current_tab: Tabs::Room,
            input: String::new(),
            homeserver_url: homeserver_url,
            full_username: full_username,
        }
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
        let message_content = event.content;
        let message = convert_message_type(message_content.msgtype, self.homeserver_url.clone());

        match self.rooms.rooms.iter_mut().find(|r| r.id == room) {
            Some(r) => {
                r.messages.add_message(
                    datetime.format("%d/%m/%Y %T").to_string(),
                    sender.clone(),
                    message.clone(),
                );
                if sender != self.full_username {
                    match notify_rust::Notification::new()
                        .summary(&sender)
                        .body(&message)
                        .icon("matrix")
                        .show()
                    {
                        Ok(_) => {}
                        Err(_) => {}
                    };
                }
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
            Tabs::Messages => match self.rooms.state.selected() {
                Some(_) => self.current_tab = Tabs::Input,
                None => self.current_tab = Tabs::Room,
            },
            Tabs::Input => {
                self.current_tab = Tabs::Members;
            }
            Tabs::Members => {
                match self.rooms.get_current_room() {
                    Some(r) => {
                        r.members.state.select(None);
                    }
                    None => {}
                }
                self.current_tab = Tabs::Room;
            }
        }
    }
}
