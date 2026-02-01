
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use regex::Regex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListenerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub pattern: Regex,
    pub target: String,
}

impl Serialize for RouteEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("RouteEntry", 2)?;
        state.serialize_field("pattern", &self.pattern.as_str())?;
        state.serialize_field("target", &self.target)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RouteEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RouteEntryHelper {
            pattern: String,
            target: String,
        }

        let helper = RouteEntryHelper::deserialize(deserializer)?;
        let regex = Regex::new(&helper.pattern).map_err(serde::de::Error::custom)?;
        Ok(RouteEntry {
            pattern: regex,
            target: helper.target,
        })
    }
}

impl RouteEntry {
    pub fn new(pattern: &str, target: &str) -> Result<Self, regex::Error> {
        let regex = Regex::new(pattern)?;
        Ok(RouteEntry {
            pattern: regex,
            target: target.to_string(),
        })
    }

    pub fn get_target(&self, input: &str) -> Option<String> {
        if let Some(caps) = self.pattern.captures(input){
            // 捕获所有并使用target模板生成目标字符串
            let mut result = self.target.clone();
            for (i, cap) in caps.iter().enumerate() {
                if let Some(m) = cap {
                    let placeholder = format!("{{{}}}", i);
                    result = result.replace(&placeholder, m.as_str());
                }
            }
            Some(result)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteTable {
    pub enabled: bool,
    pub entries: Vec<RouteEntry>,
}

impl RouteTable {
    pub fn find_target(&self, input: &str) -> Option<String> {
        if self.enabled {
            for entry in &self.entries {
                if let Some(target) = entry.get_target(input) {
                    return Some(target);
                }
            }
            return None;
        }
        Some(format!("default:{}", input))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub auth_key: String,
    pub listener: ListenerConfig,
    pub redis: Option<RedisConfig>,
    pub route_table: Option<RouteTable>,
}
