use clap::Parser;

use matrix_sdk::{
    ruma::events::{room::message::MessageEventContent, SyncMessageEvent},
    Client, Result, SyncSettings,
};
use url::Url;

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let homeserverurl = Url::parse(&args.home_server)?;
    let client = Client::new(homeserverurl)?;
    // First we need to log in.
    client
        .login(&args.username, &args.password, None, None)
        .await?;
    println!("Successfully logged in");
    client
        .register_event_handler(|ev: SyncMessageEvent<MessageEventContent>| async move {
            println!(
                "Received a message {:?} from {:?}",
                ev.content.msgtype, ev.sender
            );
        })
        .await;

    // Syncing is important to synchronize the client state with the server.
    // This method will never return.
    client.sync(SyncSettings::default()).await;

    Ok(())
}
