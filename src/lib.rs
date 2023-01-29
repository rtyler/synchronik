use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct Capability {
    name: String,
    path: PathBuf,
    data: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct CapsRequest {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct CapsResponse {
    caps: Vec<Capability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Command {
    pub script: String,
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
