use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::AppState;

/*
 * Representation of the Synchronik YAML format
 */
#[derive(Clone, Debug, Deserialize)]
pub struct Yml {
    pub needs: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scm {
    GitHub {
        owner: String,
        repo: String,
        #[serde(rename = "ref")]
        scm_ref: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct Project {
    description: String,
    pub filename: String,
    #[serde(with = "serde_yaml::with::singleton_map")]
    pub scm: Scm,
}

/*
 * Internal representation of an Agent that has been "loaded" by the server
 *
 * Loaded meaning the server has pinged the agent and gotten necessary bootstrap
 * information
 */
#[derive(Clone, Debug, Serialize)]
pub struct Agent {
    pub name: String,
    pub url: Url,
    pub capabilities: Vec<synchronik::Capability>,
}

impl Agent {
    pub fn new(name: String, url: Url, capabilities: Vec<synchronik::Capability>) -> Self {
        Self {
            name,
            url,
            capabilities,
        }
    }

    pub fn render_compact(&self, _state: &AppState<'_>) -> String {
        "".into()
        //let data = serde_json::to_str(self).unwrap_or(serde_json::Value::Array);

        //state.render("views/components/agent/compact.hbs",
        //             data: data).await.unwrap_or("".into())
    }

    pub fn can_meet(&self, needs: &Vec<String>) -> bool {
        // TODO: Improve the performance of this by reducing the clones
        let mut needs = needs.clone();
        needs.sort();

        let mut capabilities: Vec<String> = self
            .capabilities
            .iter()
            .map(|c| c.name.to_lowercase())
            .collect();
        capabilities.sort();
        capabilities == needs
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    pub url: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub agents: HashMap<String, AgentConfig>,
    pub projects: HashMap<String, Project>,
}

impl ServerConfig {
    pub fn has_project(&self, name: &str) -> bool {
        self.projects.contains_key(name)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            agents: HashMap::default(),
            projects: HashMap::default(),
        }
    }
}
