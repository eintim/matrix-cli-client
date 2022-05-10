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
        tx: Sender<(OriginalSyncRoomMessageEvent, Room)>,
    ) -> Result<Client, Error>;
    async fn send_message(&self, room_id: &String, message: String);
}

#[async_trait]
impl ClientExt for Client {
    async fn initialize(
        home_server: Url,
        username: String,
        password: String,
        tx: Sender<(OriginalSyncRoomMessageEvent, Room)>,
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
        //Event Handler
        client
            .register_event_handler({
                let tx = tx.clone();
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
            Err(err) => return Err(err),
        };

        let sync_client = client.clone();
        tokio::spawn(async move {
            sync_client.sync(SyncSettings::default()).await;
        });

        return Ok(client);
    }

    async fn send_message(&self, room_id: &String, message: String) {
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
        match room.send(content, None).await {
            Ok(_) => (),
            Err(_) => (),
        };
    }
}

pub fn convert_message_type(msgtype: MessageType) -> String {
    match msgtype {
        MessageType::Text(content) => content.body,
        MessageType::Audio(content) => {
            "Has send audio: ".to_string()
                + &content.body
                + &" ".to_string()
                + &handle_media_source(content.source)
        }
        //MessageType::Emote(content) => "Has send Sticker: ".to_string() + &content.body,
        MessageType::File(content) => {
            "Has send file: ".to_string()
                + &content.body
                + &" ".to_string()
                + &handle_media_source(content.source)
        }
        MessageType::Image(content) => {
            "Has send image: ".to_string()
                + &content.body
                + &" ".to_string()
                + &handle_media_source(content.source)
        }
        MessageType::Video(content) => {
            "Has send video: ".to_string()
                + &content.body
                + &" ".to_string()
                + &handle_media_source(content.source)
        }
        MessageType::Location(content) => "Has send location: ".to_string() + &content.geo_uri,
        _ => "Unknown messagetype".to_string(),
    }
}

fn handle_media_source(source: MediaSource) -> String {
    match source {
        MediaSource::Plain(mxc) => convert_mxc_to_url(mxc).to_string(),
        MediaSource::Encrypted(_) => "".to_string(),
    }
}

fn convert_mxc_to_url(mxc: OwnedMxcUri) -> Url {
    let mut url = Url::parse(
        "https://chat.eintim.de/_matrix/media/r0/download/eintim.de/pAopWnbWowuYUWZcFLpgQoSN",
    )
    .unwrap();

    return url;
}
