use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Capability {
    pub name: String,
    path: PathBuf,
    data: serde_json::Value,
}

impl Capability {
    pub fn with_name(name: &str) -> Self {
        Capability {
            name: name.into(),
            path: PathBuf::new(),
            data: serde_json::Value::Null,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct CapsRequest {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CapsResponse {
    pub caps: Vec<Capability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Command {
    pub script: String,
}

impl Command {
    pub fn with_script(script: &str) -> Self {
        Self {
            script: script.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandRequest {
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandResponse {
    pub uuid: Uuid,
    pub stream: Option<Url>,
    pub task: Option<Url>,
    pub log: Url,
}

#[cfg(test)]
mod tests {}
