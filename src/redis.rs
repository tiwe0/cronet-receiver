use std::{time::Duration};

use fred::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{config::AppConfig};

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveMessage {
    pub id: String,
    pub tag: String,
    pub tag_app: String,
    pub tag_url: String,
    pub data: serde_json::Value,
}

pub async fn setup_redis(app_config: &AppConfig) -> Result<Client, Error> {
    let host = app_config.redis.as_ref().unwrap().host.clone();
    let port = app_config.redis.as_ref().unwrap().port;
    let url = format!("redis://{}:{}", host, port);
    let config = Config::from_url(&url).unwrap();
    let client = Builder::from_config(config).with_connection_config(|config|{
        config.connection_timeout = Duration::from_secs(300);
        config.tcp = TcpConfig {
            nodelay: Some(true),
            ..Default::default()
        };
    }).build()?;
    client.init().await?;
    client.on_error(|(error, server)| async move {
        println!("Redis error on server {:?}: {}", server, error);
        Ok(())
    });
    Ok(client)
}

pub async fn dispatch_to_redis(client: &Client, key: String, message: &SaveMessage) -> anyhow::Result<()> {
    let json_data = serde_json::to_string(&message.data)?;

    let _: () = client.xadd(
        key,
        false,
        ("MAXLEN", "~", 1000),
        "*",
        ("data", json_data.as_str()),
    ).await?;

    Ok(())
}
