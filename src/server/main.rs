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

#[derive(Clone, Debug)]
pub struct AppState<'a> {
    pub db: SqlitePool,
    pub config: ServerConfig,
    pub agents: Vec<Agent>,
    hb: Arc<RwLock<Handlebars<'a>>>,
}

impl AppState<'_> {
    fn new(db: SqlitePool, config: ServerConfig) -> Self {
        Self {
            db,
            config,
            agents: vec![],
            hb: Arc::new(RwLock::new(Handlebars::new())),
        }
    }

    pub async fn register_templates(&self) -> Result<(), handlebars::TemplateFileError> {
        let mut hb = self.hb.write().await;
        hb.clear_templates();
        hb.register_templates_directory(".hbs", "views")
    }

    pub async fn render(
        &self,
        name: &str,
        data: &serde_json::Value,
    ) -> Result<tide::Body, tide::Error> {
        /*
         * In debug mode, reload the templates on ever render to avoid
         * needing a restart
         */
        #[cfg(debug_assertions)]
        {
            self.register_templates().await;
        }
        let hb = self.hb.read().await;
        let view = hb.render(name, data)?;
        Ok(tide::Body::from_string(view))
    }
}

/**
 * The routes module contains all the tide routes and the logic to fulfill the responses for each
 * route.
 *
 * Modules are nested for cleaner organization here
 */
mod routes {
    use crate::AppState;
    use log::*;
    use tide::{Body, Request};

    /**
     *  GET /
     */
    pub async fn index(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        let params = json!({
            "page": "home",
            "config" : req.state().config,
        });

        let mut body = req.state().render("index", &params).await?;
        body.set_mime("text/html");
        Ok(body)
    }

    pub mod api {
        use crate::{AppState, JankyYml, Scm};
        use log::*;
        use tide::{Body, Request, Response, StatusCode};

        /**
         *  POST /projects/{name}
         */
        pub async fn execute_project(req: Request<AppState<'_>>) -> Result<Response, tide::Error> {
            let name: String = req.param("name")?.into();
            let state = req.state();

            if !state.config.has_project(&name) {
                debug!("Could not find project named: {}", name);
                return Ok(Response::new(StatusCode::NotFound));
            }

            if let Some(project) = state.config.projects.get(&name) {
                match &project.scm {
                    Scm::GitHub {
                        owner,
                        repo,
                        scm_ref,
                    } => {
                        debug!(
                            "Fetching the file {} from {}/{}",
                            &project.filename, owner, repo
                        );
                        let res = octocrab::instance()
                            .repos(owner, repo)
                            .raw_file(
                                octocrab::params::repos::Commitish(scm_ref.into()),
                                &project.filename,
                            )
                            .await?;
                        let jankyfile: JankyYml = serde_yaml::from_str(&res.text().await?)?;
                        debug!("text: {:?}", jankyfile);

                        for agent in &state.agents {
                            if agent.can_meet(&jankyfile.needs) {
                                debug!("agent: {:?} can meet our needs", agent);
                                let commands: Vec<janky::Command> = jankyfile
                                    .commands
                                    .iter()
                                    .map(|c| janky::Command::with_script(c))
                                    .collect();
                                let commands = janky::CommandRequest { commands };
                                let client = reqwest::Client::new();
                                let res = client
                                    .put(
                                        agent
                                            .url
                                            .join("/api/v1/execute")
                                            .expect("Failed to join execute URL"),
                                    )
                                    .json(&commands)
                                    .send()
                                    .await?;

                                return Ok(json!({
                                    "msg": format!("Executing on {}", &agent.url)
                                })
                                .into());
                            }
                        }
                    }
                }
                return Ok("{}".into());
            }
            Ok(Response::new(StatusCode::InternalServerError))
        }
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
    #[serde(with = "serde_yaml::with::singleton_map")]
    scm: Scm,
    filename: String,
}

/*
 * Internal representation of an Agent that has been "loaded" by the server
 *
 * Loaded meaning the server has pinged the agent and gotten necessary bootstrap
 * information
 */
#[derive(Clone, Debug, Serialize)]
pub struct Agent {
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
pub struct ServerConfig {
    agents: Vec<Url>,
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
            agents: vec![],
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
    let mut state = AppState::new(pool, config.clone());

    for url in &config.agents {
        debug!("Requesting capabilities from agent: {}", url);
        let response: janky::CapsResponse = reqwest::get(url.join("/api/v1/capabilities")?)
            .await?
            .json()
            .await?;
        state.agents.push(Agent {
            url: url.clone(),
            capabilities: response.caps,
        });
    }

    state.register_templates().await;
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
            url: Url::parse("http://localhost").unwrap(),
            capabilities,
        };
        assert!(agent.can_meet(&needs));
    }
}
