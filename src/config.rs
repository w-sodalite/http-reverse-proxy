use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    server: Server,

    #[serde(default)]
    client: Client,

    #[serde(default)]
    logging: Logging,

    #[serde(default)]
    routes: Vec<Route>,
}

impl Config {
    pub fn from_yaml<P: AsRef<Path>>(p: P) -> anyhow::Result<Config> {
        let file = File::open(p).map_err(|e| anyhow!(e))?;
        serde_yaml::from_reader::<_, Config>(file).map_err(|e| anyhow!(e))
    }

    pub fn into_parts(self) -> (Server, Client, Logging, Vec<Route>) {
        let Config {
            server,
            client,
            logging,
            routes,
        } = self;
        (server, client, logging, routes)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Server {
    port: u16,
}

impl Server {
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Client {
    pool_max_idle_per_host: usize,
    pool_idle_timeout: Option<Duration>,
}

impl Client {
    pub fn pool_max_idle_per_host(&self) -> usize {
        self.pool_max_idle_per_host
    }
    pub fn pool_idle_timeout(&self) -> Option<Duration> {
        self.pool_idle_timeout
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Logging {
    level: HashMap<String, String>,
}

impl Logging {
    pub fn level(&self) -> &HashMap<String, String> {
        &self.level
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Route {
    id: String,

    uri: String,

    predicate: String,

    #[serde(default)]
    strip: usize,
}

impl Route {
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    pub fn strip(&self) -> usize {
        self.strip
    }
}
