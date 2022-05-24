use anyhow::anyhow;

use crate::config::Config;
use crate::server::Server;

mod config;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_yaml("./config.yaml").map_err(|e| anyhow!(e))?;
    let server = Server::with_config(config);
    server.serve().await
}
