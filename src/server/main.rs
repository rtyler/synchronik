/*
 * This is the main Janky entrypoint for the server"
 */

#[macro_use]
extern crate serde_json;

use std::collections::HashMap;
use std::path::PathBuf;

use async_std::sync::{Arc, RwLock};
use dotenv::dotenv;
use gumdrop::Options;
use handlebars::Handlebars;
use log::*;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use url::Url;

mod dao;
mod routes;

#[derive(Clone, Debug)]
pub struct AppState<'a> {
    pub db: SqlitePool,
    pub config: ServerConfig,
    pub agents: Vec<Agent>,
    hb: Arc<RwLock<Handlebars<'a>>>,
}

impl AppState<'_> {
    fn new(db: SqlitePool, config: ServerConfig) -> Self {
        let mut hb = Handlebars::new();

        #[cfg(debug_assertions)]
        hb.set_dev_mode(true);

        Self {
            db,
            config,
            agents: vec![],
            hb: Arc::new(RwLock::new(hb)),
        }
    }

    pub async fn register_templates(&self) -> Result<(), handlebars::TemplateError> {
        let mut hb = self.hb.write().await;
        hb.clear_templates();
        hb.register_templates_directory(".hbs", "views")
    }

    pub async fn render(
        &self,
        name: &str,
        data: &serde_json::Value,
    ) -> Result<tide::Body, tide::Error> {
        let hb = self.hb.read().await;
        let view = hb.render(name, data)?;
        Ok(tide::Body::from_string(view))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct JankyYml {
    needs: Vec<String>,
    commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Scm {
    GitHub {
        owner: String,
        repo: String,
        #[serde(rename = "ref")]
        scm_ref: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
struct Project {
    description: String,
    filename: String,
    #[serde(with = "serde_yaml::with::singleton_map")]
    scm: Scm,
}

/*
 * Internal representation of an Agent that has been "loaded" by the server
 *
 * Loaded meaning the server has pinged the agent and gotten necessary bootstrap
 * information
 */
#[derive(Clone, Debug, Serialize)]
pub struct Agent {
    name: String,
    url: Url,
    capabilities: Vec<janky::Capability>,
}

impl Agent {
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
struct AgentConfig {
    url: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    agents: HashMap<String, AgentConfig>,
    projects: HashMap<String, Project>,
}

impl ServerConfig {
    fn has_project(&self, name: &str) -> bool {
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

#[derive(Debug, Options)]
struct ServerOptions {
    #[options(help = "print help message")]
    help: bool,
    #[options(help = "host:port to bind the server to", default = "0.0.0.0:8000")]
    listen: String,
    #[options(help = "Path to the configuration file")]
    config: Option<PathBuf>,
    #[options(help = "Comma separated list of URLs for agents")]
    agents: Vec<Url>,
}

#[async_std::main]
async fn main() -> Result<(), tide::Error> {
    pretty_env_logger::init();
    dotenv().ok();
    let opts = ServerOptions::parse_args_default_or_exit();
    debug!("Starting with options: {:?}", opts);

    let config = match opts.config {
        Some(path) => {
            let config_file = std::fs::File::open(path).expect("Failed to open config file");
            serde_yaml::from_reader(config_file).expect("Failed to read config file")
        }
        None => ServerConfig::default(),
    };
    debug!("Starting with config: {:?}", config);

    let database_url = std::env::var("DATABASE_URL").unwrap_or(":memory:".to_string());
    let pool = SqlitePool::connect(&database_url).await?;

    /* If janky-server is running in memory, make sure the database is set up properly */
    if database_url == ":memory:" {
        sqlx::migrate!().run(&pool).await?;
    }
    let mut state = AppState::new(pool.clone(), config.clone());

    /*
     * Make sure the database has all the projects configured
     */

    for name in config.projects.keys() {
        match dao::Project::by_name(&name, &pool).await {
            Ok(_) => {}
            Err(sqlx::Error::RowNotFound) => {
                debug!("Project not found in database, creating: {}", name);
                dao::Project::create(&dao::Project::new(&name), &pool).await?;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    for (name, agent) in config.agents.iter() {
        debug!("Requesting capabilities from agent: {:?}", agent);
        let response: janky::CapsResponse = reqwest::get(agent.url.join("/api/v1/capabilities")?)
            .await?
            .json()
            .await?;
        state.agents.push(Agent {
            name: name.clone(),
            url: agent.url.clone(),
            capabilities: response.caps,
        });
    }

    state
        .register_templates()
        .await
        .expect("Failed to register handlebars templates");
    let mut app = tide::with_state(state);

    #[cfg(not(debug_assertions))]
    {
        info!("Activating RELEASE mode configuration");
        app.with(driftwood::ApacheCombinedLogger);
    }

    #[cfg(debug_assertions)]
    {
        info!("Activating DEBUG mode configuration");
        info!("Enabling a very liberal CORS policy for debug purposes");
        use tide::security::{CorsMiddleware, Origin};
        let cors = CorsMiddleware::new()
            .allow_methods(
                "GET, POST, PUT, OPTIONS"
                    .parse::<tide::http::headers::HeaderValue>()
                    .unwrap(),
            )
            .allow_origin(Origin::from("*"))
            .allow_credentials(false);

        app.with(cors);
    }
    /*
     * All builds will have apidocs, since they're handy
     */
    app.at("/apidocs").serve_dir("apidocs/")?;
    app.at("/static").serve_dir("static/")?;

    debug!("Configuring routes");
    app.at("/").get(routes::index);
    app.at("/project/:name").get(routes::project);

    debug!("Configuring API routes");
    app.at("/api/v1/projects/:name")
        .post(routes::api::execute_project);
    app.listen(opts.listen).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use janky::*;

    #[test]
    fn agent_can_meet_false() {
        let needs: Vec<String> = vec!["rspec".into(), "git".into(), "dotnet".into()];
        let capabilities = vec![Capability::with_name("rustc")];
        let agent = Agent {
            name: "test".into(),
            url: Url::parse("http://localhost").unwrap(),
            capabilities,
        };
        assert_eq!(false, agent.can_meet(&needs));
    }

    #[test]
    fn agent_can_meet_true() {
        let needs: Vec<String> = vec!["dotnet".into()];
        let capabilities = vec![Capability::with_name("dotnet")];
        let agent = Agent {
            name: "test".into(),
            url: Url::parse("http://localhost").unwrap(),
            capabilities,
        };
        assert!(agent.can_meet(&needs));
    }

    #[test]
    fn agent_can_meet_false_multiple() {
        let needs: Vec<String> = vec!["rspec".into(), "git".into(), "dotnet".into()];
        let capabilities = vec![Capability::with_name("dotnet")];
        let agent = Agent {
            name: "test".into(),
            url: Url::parse("http://localhost").unwrap(),
            capabilities,
        };
        assert_eq!(false, agent.can_meet(&needs));
    }

    #[test]
    fn agent_can_meet_true_multiple() {
        let needs: Vec<String> = vec!["rspec".into(), "dotnet".into()];
        let capabilities = vec![
            Capability::with_name("dotnet"),
            Capability::with_name("rspec"),
        ];
        let agent = Agent {
            name: "test".into(),
            url: Url::parse("http://localhost").unwrap(),
            capabilities,
        };
        assert!(agent.can_meet(&needs));
    }
}
