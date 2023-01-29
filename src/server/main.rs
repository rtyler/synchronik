/*
 * This is the main Janky entrypoint for the server"
 */

#[macro_use]
extern crate serde_json;

use std::path::PathBuf;

use async_std::sync::{Arc, RwLock};
use dotenv::dotenv;
use gumdrop::Options;
use handlebars::Handlebars;
use log::*;
use serde::Deserialize;
use sqlx::SqlitePool;
use url::Url;

#[derive(Clone, Debug)]
pub struct AppState<'a> {
    pub db: SqlitePool,
    hb: Arc<RwLock<Handlebars<'a>>>,
}

impl AppState<'_> {
    fn new(db: SqlitePool) -> Self {
        Self {
            db,
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
            "page": "home"
        });

        let res = octocrab::instance()
            .repos("rtyler", "janky")
            .raw_file(
                octocrab::params::repos::Commitish("main".into()),
                "Jankyfile",
            )
            .await?;

        debug!("jank: {:?}", res);
        debug!("text: {:?}", res.text().await?);

        let mut body = req.state().render("index", &params).await?;
        body.set_mime("text/html");
        Ok(body)
    }

    pub mod api {}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Scm {
    GitHub,
}

#[derive(Clone, Debug, Deserialize)]
struct Project {
    #[serde(rename = "type")]
    scm_type: Scm,
    url: Url,
    #[serde(rename = "ref")]
    scm_ref: String,
    filename: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
struct ServerConfig {
    agents: Vec<Url>,
    projects: Vec<Project>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            agents: vec![],
            projects: vec![],
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

    let state = AppState::new(pool);
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
    app.listen(opts.listen).await?;
    Ok(())
}
