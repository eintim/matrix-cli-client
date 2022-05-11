use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::{
        events::room::{
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
            MediaSource,
        },
        OwnedMxcUri, RoomId,
    },
    Client, Error,
};
use url::Url;

use tokio::sync::mpsc::Sender;

use async_trait::async_trait;

#[async_trait]
pub trait ClientExt {
    async fn initialize(
        home_server: Url,
        username: String,
        password: String,
        tx: Sender<(OriginalSyncRoomMessageEvent, Room, Client)>,
    ) -> Result<Client, Error>;
    async fn send_message(&self, room_id: &str, message: String);
}

#[async_trait]
impl ClientExt for Client {
    /// Initialize the matrix client
    /// # Arguments
    /// * `home_server` - The homeserver url
    /// * `username` - The username
    /// * `password` - The password
    /// * `tx` - The channel to send message events to
    async fn initialize(
        home_server: Url,
        username: String,
        password: String,
        tx: Sender<(OriginalSyncRoomMessageEvent, Room, Client)>,
    ) -> Result<Client, Error> {
        let client = match Client::new(home_server).await {
            Ok(client) => client,
            Err(err) => {
                return Err(Error::Http(err));
            }
        };

        match client
            .login(&username, &password, None, Some("Matrix-Tui-Client"))
            .await
        {
            Ok(_) => (),
            Err(err) => return Err(err),
        };

        match client.sync_once(SyncSettings::default()).await {
            Ok(_) => (),
            Err(err) => return Err(err),
        };

        // Register Event Handler
        client
            .register_event_handler({
                let tx = tx.clone();
                move |ev: OriginalSyncRoomMessageEvent, room: Room, client: Client| {
                    let tx = tx.clone();
                    async move {
                        if (tx.send((ev, room, client)).await).is_ok() {};
                    }
                }
            })
            .await;

        // Clone client to endlessly sync with server to get events
        let sync_client = client.clone();
        tokio::spawn(async move {
            sync_client.sync(SyncSettings::default()).await;
        });

        return Ok(client);
    }

    /// Send a message to a room
    /// # Arguments
    /// * `room_id` - The room id
    /// * `message` - The message to send
    async fn send_message(&self, room_id: &str, message: String) {
        if message.is_empty() {
            return;
        }

        let room_id = match RoomId::parse(room_id) {
            Ok(room_id) => room_id,
            Err(_) => return,
        };
        let room = match self.get_joined_room(&room_id) {
            Some(room) => room,
            None => return,
        };
        let content = RoomMessageEventContent::text_plain(message);
        if (room.send(content, None).await).is_ok() {};
    }
}

/// Convert MessageType to a readable string
///
/// # Arguments
/// * `message_type` - The message type
/// * `homeserver_url` - The homeserver url
pub fn convert_message_type(msgtype: MessageType, homeserver_url: Url) -> String {
    match msgtype {
        MessageType::Text(content) => content.body,
        MessageType::Audio(content) => {
            "Has send audio: ".to_string()
                + &content.body
                + " "
                + &handle_media_source(content.source, homeserver_url)
        }
        MessageType::File(content) => {
            "Has send file: ".to_string()
                + &content.body
                + " "
                + &handle_media_source(content.source, homeserver_url)
        }
        MessageType::Image(content) => {
            "Has send image: ".to_string()
                + &content.body
                + " "
                + &handle_media_source(content.source, homeserver_url)
        }
        MessageType::Video(content) => {
            "Has send video: ".to_string()
                + &content.body
                + " "
                + &handle_media_source(content.source, homeserver_url)
        }
        MessageType::Location(content) => "Has send location: ".to_string() + &content.geo_uri,
        _ => "Unknown messagetype".to_string(),
    }
}

fn handle_media_source(source: MediaSource, homeserver_url: Url) -> String {
    match source {
        MediaSource::Plain(mxc) => convert_mxc_to_url(mxc, homeserver_url).to_string(),
        MediaSource::Encrypted(_) => "".to_string(),
    }
}

fn convert_mxc_to_url(mxc: OwnedMxcUri, mut base_url: Url) -> Url {
    match mxc.parts() {
        Ok((server_name, media_id)) => {
            base_url.set_path(&format!(
                "/_matrix/media/r0/download/{}/{}",
                server_name, media_id
            ));
            base_url
        }
        Err(_) => base_url,
    }
}
