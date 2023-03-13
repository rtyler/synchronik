use std::collections::HashMap;
use std::path::PathBuf;

use log::*;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::AppState;

/*
 * Representation of the Synchronik YAML format
 */
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Yml {
    pub needs: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scm {
    /*
     * The Nonexistent Scm is used for stubbing out the Scm properties when
     * inlining configuration
     */
    Nonexistent,
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
    /*
     * Used for optionally defining an inline Yml configuration
     */
    inline: Option<Yml>,
    pub filename: Option<String>,
    #[serde(default = "default_scm", with = "serde_yaml::with::singleton_map")]
    pub scm: Scm,
}

/*
 * Simple default scm for use when nothing has been otherwise defined
 */
fn default_scm() -> Scm {
    Scm::Nonexistent
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
        self.name.clone()
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServerConfig {
    pub agents: HashMap<String, AgentConfig>,
    pub projects: HashMap<String, Project>,
}

impl ServerConfig {
    pub fn has_project(&self, name: &str) -> bool {
        self.projects.contains_key(name)
    }

    /*
     * Load the ServerConfig from the given file.
     */
    fn from_filepath(path: &PathBuf) -> anyhow::Result<Self> {
        let config_file = std::fs::File::open(path).expect("Failed to open config file");
        serde_yaml::from_reader(config_file).map_err(anyhow::Error::from)
    }

    /*
     * Load the ServerConfig from an amalgamation of yaml in the given directory
     */
    fn from_dirpath(path: &PathBuf) -> anyhow::Result<Self> {
        use glob::glob;
        use std::fs::File;

        let pattern = format!("{}/**/*.yml", path.as_path().to_string_lossy());
        debug!("Loading config from directory with pattern: {}", pattern);

        let mut values: Vec<serde_yaml::Value> = vec![];

        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    if let Ok(file) = File::open(path) {
                        if let Ok(value) = serde_yaml::from_reader(file) {
                            values.push(value);
                        }
                    }
                }
                Err(e) => error!("Failed to read entry: {:?}", e),
            }
        }

        /*
         * At this point we should have enough partials to do a coercion to the ServerConfig
         * structure
         */
        let mut v = serde_yaml::Value::Null;
        for m in values.drain(0..) {
            merge_yaml(&mut v, m);
        }
        serde_yaml::from_value(v).map_err(anyhow::Error::from)
    }

    /*
     * Take the given path and do the necessary deserialization whether a file or a directory
     */
    pub fn from_path(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            error!("The provided configuration path does not exist: {:?}", path);
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }

        match path.is_file() {
            true => Self::from_filepath(&path),
            false => Self::from_dirpath(&path),
        }
    }
}

/*
 * Merge two Valus from <https://stackoverflow.com/a/67743348>
 */
fn merge_yaml(a: &mut serde_yaml::Value, b: serde_yaml::Value) {
    match (a, b) {
        (a @ &mut serde_yaml::Value::Mapping(_), serde_yaml::Value::Mapping(b)) => {
            let a = a.as_mapping_mut().unwrap();
            for (k, v) in b {
                if v.is_sequence() && a.contains_key(&k) && a[&k].is_sequence() {
                    let mut _b = a.get(&k).unwrap().as_sequence().unwrap().to_owned();
                    _b.append(&mut v.as_sequence().unwrap().to_owned());
                    a[&k] = serde_yaml::Value::from(_b);
                    continue;
                }
                if !a.contains_key(&k) {
                    a.insert(k.to_owned(), v.to_owned());
                } else {
                    merge_yaml(&mut a[&k], v);
                }
            }
        }
        (a, b) => *a = b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_serverconfig_from_filepath() {
        let path = PathBuf::from("./examples/server.yml");
        let config = ServerConfig::from_path(&path);
        match config {
            Ok(config) => {
                assert_eq!(
                    config.agents.len(),
                    1,
                    "Unexpected number of agents: {:?}",
                    config.agents
                );
            }
            Err(e) => {
                assert!(false, "Failed to process ServerConfig: {:?}", e);
            }
        }
    }

    #[test]
    fn test_serverconfig_non0xistent() {
        let path = PathBuf::from("./non-existing/path/withstuff");
        let config = ServerConfig::from_path(&path);
        assert!(config.is_err());
    }

    #[test]
    fn test_serverconfig_from_filedir() {
        let path = PathBuf::from("./examples/synchronik.d");
        let config = ServerConfig::from_path(&path);
        match config {
            Ok(config) => {
                assert_eq!(
                    config.agents.len(),
                    1,
                    "Unexpected number of agents: {:?}",
                    config.agents
                );
                assert_eq!(
                    config.projects.len(),
                    2,
                    "Unexpected number of projects: {:?}",
                    config.projects
                );
            }
            Err(e) => {
                assert!(false, "Failed to process ServerConfig: {:?}", e);
            }
        }
    }

    #[test]
    fn parse_config_with_scm() {
        let conf = r#"
---
agents:
  'Local':
    url: 'http://localhost:9000'
projects:
  'synchronik':
    description: |
      Self-hosted project
    filename: 'ci.synchronik.yml'
    scm:
      github:
        owner: 'rtyler'
        repo: 'synchronik'
        ref: 'main'
"#;
        let value: ServerConfig = serde_yaml::from_str(&conf).expect("Failed to parse");
        assert_eq!(value.agents.len(), 1);
    }

    #[test]
    fn parse_config_inline() {
        let conf = r#"
---
agents:
  'Local':
    url: 'http://localhost:9000'
projects:
  'synchronik':
    description: |
      Self-hosted project
    inline:
      needs:
        - git
      commands:
        - 'whoami'
"#;
        let value: ServerConfig = serde_yaml::from_str(&conf).expect("Failed to parse");
        assert_eq!(value.agents.len(), 1);
        assert_eq!(value.projects.len(), 1);

        let project = value.projects.get("synchronik").unwrap();
        match &project.inline {
            Some(yml) => {
                assert!(yml.commands.contains(&"whoami".to_string()));
            }
            None => {
                assert!(false);
            }
        }
    }
}
