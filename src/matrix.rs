use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::{
        events::room::message::{OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        RoomId,
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
