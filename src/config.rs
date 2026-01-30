use std::collections::HashMap;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListenerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub auth_key: String,
    pub listener: ListenerConfig,
    pub redis: Option<RedisConfig>,
    pub route_table: Option<HashMap<String, String>>,
}
