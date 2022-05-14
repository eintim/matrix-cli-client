use crate::matrix::convert_message_type;
use futures::{pin_mut, StreamExt};

use crate::matrix::*;
use matrix_sdk::{
    room::Room as MatrixRoom,
    ruma::events::{
        room::{
            member::{MembershipState, OriginalSyncRoomMemberEvent},
            message::OriginalSyncRoomMessageEvent,
        },
        AnySyncMessageLikeEvent, AnySyncRoomEvent, SyncMessageLikeEvent,
    },
    Client, RoomType,
};

use tui::widgets::ListState;

use chrono::offset::Utc;
use chrono::DateTime;

use std::time::SystemTime;
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
    /// Create a new ScrollableMessageList
    pub fn new() -> ScrollableMessageList {
        ScrollableMessageList {
            state: ListState::default(),
            messages: Vec::new(),
            mode: MessageViewMode::Follow,
        }
    }

    /// Create a new ScrollableMessageList with the given messages.
    pub fn with_messages(messages: Vec<(String, String, String)>) -> ScrollableMessageList {
        let mut list = ScrollableMessageList {
            state: ListState::default(),
            messages,
            mode: MessageViewMode::Follow,
        };
        list.state
            .select(Some(list.messages.len().saturating_sub(1)));
        list
    }

    /// Add a message to the list.
    /// If Follow mode is active, the cursor will be moved to the newest message
    /// # Arguments
    /// * `time` - The time the message was sent.
    /// * `sender` - The sender of the message.
    /// * `message` - The message.
    pub fn add_message(&mut self, time: String, sender: String, message: String) {
        self.messages.push((time, sender, message));

        if self.mode == MessageViewMode::Follow {
            self.state.select(Some(self.messages.len() - 1));
        }
    }

    /// Change the selected message to the next one
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

    /// Change the selected message to the previous one
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
    pub members: Vec<(String, String)>,
}

impl ScrollableMemberList {
    /// Create a new member list
    ///
    /// # Arguments
    /// * `members` - A vector of member names
    pub fn with_members(members: Vec<(String, String)>) -> ScrollableMemberList {
        ScrollableMemberList {
            state: ListState::default(),
            members,
        }
    }

    /// Change the selected member to the next one
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

    /// Change the selected member to the previous one
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
    /// Create a new room with the given name and id.
    /// Gets past members and past messages from the room.
    ///
    /// # Arguments
    /// * `name` - The room to create.
    /// * `homeserver_url` - The homeserver url.
    pub async fn new(room: MatrixRoom, homeserver_url: Url) -> Room {
        let name = match room.display_name().await {
            Ok(name) => name.to_string(),
            Err(_) => "Unknown name".to_string(),
        };

        let members = match room.joined_members().await {
            Ok(members) => members,
            Err(_) => Vec::new(),
        };

        let member_names = members
            .into_iter()
            .map(|member| match member.display_name() {
                Some(name) => (name.to_string(), member.user_id().to_string()),
                None => (member.user_id().to_string(), member.user_id().to_string()),
            })
            .collect::<Vec<(String, String)>>();

        //Get old message
        match room.timeline_backward().await {
            Ok(timeline) => {
                let mut messages: Vec<(String, String, String)> = Vec::new();

                pin_mut!(timeline);
                while let Some(event) = timeline.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => break,
                    };
                    let event = match event.event.deserialize() {
                        Ok(event) => event,
                        Err(_) => break,
                    };
                    if let AnySyncRoomEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(
                        SyncMessageLikeEvent::Original(event),
                    )) = event
                    {
                        let system_time = match event.origin_server_ts.to_system_time() {
                            Some(time) => time,
                            None => SystemTime::UNIX_EPOCH,
                        };
                        let sender = event.sender.to_string();
                        let date_time: DateTime<Utc> = system_time.into();

                        messages.push((
                            date_time.format("%d/%m/%Y %T").to_string(),
                            sender,
                            (convert_message_type(event.content.msgtype, homeserver_url.clone())
                                .to_string())
                            .to_string(),
                        ));
                    }
                }
                messages.reverse();
                Room {
                    name,
                    id: room.room_id().to_string(),
                    messages: ScrollableMessageList::with_messages(messages),
                    members: ScrollableMemberList::with_members(member_names),
                }
            }
            Err(_) => Room {
                name,
                id: room.room_id().to_string(),
                messages: ScrollableMessageList::new(),
                members: ScrollableMemberList::with_members(member_names),
            },
        }
    }
}

/// Scrollable list of rooms
pub struct ScrollableRoomList {
    pub state: ListState,
    pub rooms: Vec<Room>,
}

impl ScrollableRoomList {
    /// Create a new room list
    pub fn new() -> ScrollableRoomList {
        ScrollableRoomList {
            state: ListState::default(),
            rooms: Vec::new(),
        }
    }

    /// Add a new room to the list
    ///
    /// # Arguments
    /// * `room` - The room to add
    /// * `homeserver_url` - The homeserver url
    pub async fn add_room(&mut self, room: MatrixRoom, homeserver_url: Url) {
        let room = Room::new(room, homeserver_url).await;
        self.rooms.push(room);
    }

    /// Change the selected room to the next one
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

    /// Change the selected room to the previous one
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

    /// Returns the selected room
    pub fn get_current_room(&mut self) -> Option<&mut Room> {
        let i = match self.state.selected() {
            Some(i) => i,
            None => return None,
        };
        self.rooms.get_mut(i)
    }
}

/// Selectable tabs in the UI
#[derive(PartialEq, Eq)]
pub enum Tabs {
    Room,
    Members,
    Messages,
    Input,
}

/// The state of the application
pub struct App {
    pub rooms: ScrollableRoomList,
    pub current_tab: Tabs,
    pub input: String,
    pub client: Client,
}

impl App {
    /// Returns a new App instance with the given homeserver url and username.
    /// Load rooms from client.
    /// # Arguments
    /// * `client` - The client to use
    /// # Returns
    /// A new App instance.
    pub async fn new(client: Client) -> App {
        let mut app = App {
            rooms: ScrollableRoomList::new(),
            current_tab: Tabs::Room,
            input: String::new(),
            client,
        };
        app.load_rooms().await;
        app
    }

    /// Load the rooms from the homeserver and add them to the room list.
    async fn load_rooms(&mut self) {
        let rooms = self.client.rooms();

        for room in rooms {
            if room.room_type() == RoomType::Joined {
                self.rooms
                    .add_room(room, self.client.homeserver().await)
                    .await;
            }
        }

        // Accepts all invites
        let invites = self.client.invited_rooms();
        for room in invites {
            room.accept_invitation_background();
        }
    }

    /// Handles OriginalSyncRoomMessage events.
    /// Takes data from the event and adds it to room.
    /// Throws system notifications if the event is a message.
    /// # Arguments
    /// * `event` - The event to handle.
    /// * `room` - The room to handle the event in.
    /// * `client` - The client used to receive messages.
    pub async fn handle_matrix_message_event(
        &mut self,
        event: OriginalSyncRoomMessageEvent,
        room: MatrixRoom,
        client: Client,
    ) {
        let room = room.room_id().to_string();
        let system_time = match event.origin_server_ts.to_system_time() {
            Some(time) => time,
            None => return,
        };
        let datetime: DateTime<Utc> = system_time.into();

        let sender = event.sender.to_string();
        let message_content = event.content;
        let message = convert_message_type(message_content.msgtype, client.homeserver().await);

        match self.rooms.rooms.iter_mut().find(|r| r.id == room) {
            Some(r) => {
                r.messages.add_message(
                    datetime.format("%d/%m/%Y %T").to_string(),
                    sender.clone(),
                    message.clone(),
                );
                let current_user = match client.user_id().await {
                    Some(user_id) => user_id.to_string(),
                    None => "".to_string(),
                };
                if sender != current_user
                    && notify_rust::Notification::new()
                        .summary(&sender)
                        .body(&message)
                        .icon("matrix")
                        .show()
                        .is_ok()
                {}
            }
            None => {}
        }
    }

    /// Handles OriginalSyncRoomMemberEvent events.
    /// Takes data from the event and adds it to room.
    /// # Arguments
    /// * `event` - The event to handle.
    /// * `room` - The room to handle the event in.
    /// * `client` - The client used to receive messages.
    pub async fn handle_matrix_room_event(
        &mut self,
        event: OriginalSyncRoomMemberEvent,
        room: MatrixRoom,
        client: Client,
    ) {
        let user_id = match client.user_id().await {
            Some(user_id) => user_id,
            None => return,
        };
        if event.content.membership == MembershipState::Join {
            //Check if room is already in the list
            let room_id = room.room_id().to_string();
            match self.rooms.rooms.iter_mut().find(|r| r.id == room_id) {
                Some(r) => {
                    let display_name = match event.content.displayname {
                        Some(display_name) => display_name,
                        None => "Unknown name".to_string(),
                    };
                    //Check if member is already in the list
                    match r
                        .members
                        .members
                        .iter_mut()
                        .find(|m| m.1 == event.state_key)
                    {
                        Some(_) => {}
                        None => {
                            r.members
                                .members
                                .push((display_name, event.state_key.to_string()));
                        }
                    };
                }
                None => {
                    // Create room if client joined
                    if event.state_key == user_id {
                        self.rooms
                            .add_room(room.clone(), client.homeserver().await)
                            .await;
                    }
                }
            };
        };
        if event.content.membership == MembershipState::Leave {
            let room_id = room.room_id().to_string();
            match self.rooms.rooms.iter().position(|r| r.id == room_id) {
                Some(i) => {
                    if event.state_key == user_id {
                        // Deselect room to avoid crash
                        if self.rooms.state.selected() == Some(i) {
                            self.rooms.state.select(None);
                        }
                        self.rooms.rooms.remove(i);
                        // Reset Tab if last room is closed
                        if self.current_tab == Tabs::Members || self.current_tab == Tabs::Input {
                            self.current_tab = Tabs::Room;
                        }
                    } else {
                        let room = match self.rooms.rooms.get_mut(i) {
                            Some(room) => room,
                            None => return,
                        };
                        match room
                            .members
                            .members
                            .iter_mut()
                            // State_key contains user_id of event
                            .position(|m| m.1 == event.state_key)
                        {
                            Some(i) => {
                                // Deselect member to avoid crash
                                if room.members.state.selected() == Some(i) {
                                    room.members.state.select(None);
                                };
                                room.members.members.remove(i);
                            }
                            None => (),
                        };
                    }
                }
                None => {}
            };
        };
    }

    /// Switches to the next tab.
    /// If room is selected:
    /// Room -> Messages -> Input -> Members -> Room -> ...
    ///
    /// No room selected:
    /// Room -> WelcomeScreen -> Room -> ...
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
