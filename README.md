# Matrix CLI client

this is a very basic Matrix-CLI-client written in Rust

## Features
* Switch Rooms
* Send Messages
* See/Kick Members

## Usage
```bash
cargo run -- https://your.homeserver.de -u yourusername -p yourpassword
```
Or use the binary directly:
```bash
./matrix_client https://your.homeserver.de -u yourusername -p yourpassword
```