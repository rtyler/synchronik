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
    script: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandRequest {
    commands: Vec<Command>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandResponse {
    uuid: Uuid,
    stream_url: Option<Url>,
    task_url: Url,
}

#[cfg(test)]
mod tests {}
