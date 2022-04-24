use clap::Parser;

use matrix_sdk::Result;
use url::Url;

mod app;
mod ui;
use crate::ui::run_tui;

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

    let _homeserverurl = Url::parse(&args.home_server)?;

    //UI Stuff
    run_tui(args.username, args.password).await?;

    // create app and run it

    Ok(())
}
