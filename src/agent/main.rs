#[macro_use]
extern crate serde_json;

use std::path::PathBuf;

use dotenv::dotenv;
use log::*;

const AGENT_LOGS_DIR: &str = "agent-logs";

mod caps;

mod routes {
    use tide::{Body, Request};

    /**
     *  GET /
     */
    pub async fn index(_req: Request<()>) -> Result<Body, tide::Error> {
        Ok("Hello World from the Janky Agent".into())
    }

    pub mod api {
        use crate::caps::*;
        use crate::AGENT_LOGS_DIR;
        use janky::{CommandRequest, CommandResponse};
        use log::*;
        use tide::{Body, Request};
        use url::Url;
        use uuid::Uuid;

        use std::path::Path;

        pub fn register(app: &mut tide::Server<()>) {
            app.at("/api/v1/capabilities").get(get_caps);
            app.at("/api/v1/execute").put(execute);
        }

        /*
         * PUT /execute
         *
         * This will take in the commands to actually execute
         */
        pub async fn execute(mut req: Request<()>) -> Result<Body, tide::Error> {
            let c: CommandRequest = req.body_json().await?;
            debug!("Commands to exec: {:?}", c);
            let uuid = Uuid::new_v4();
            // Create my log directory
            let log_dir = Path::new(AGENT_LOGS_DIR).join(uuid.hyphenated().to_string());
            // TODO: Handle this error
            std::fs::create_dir(log_dir.clone());

            let log_file_path = log_dir.join("console.log");
            let log_file = std::fs::File::create(log_file_path.clone()).unwrap();
            let mut bufw = std::io::BufWriter::new(log_file);

            for command in c.commands.iter() {
                use os_pipe::pipe;
                use std::io::{BufRead, BufReader, Write};
                use std::process::Command;
                let mut cmd = Command::new("sh");
                cmd.args(["-xec", &command.script]);
                let (mut reader, writer) = pipe().unwrap();
                let writer_clone = writer.try_clone().unwrap();
                cmd.stdout(writer);
                cmd.stderr(writer_clone);
                let mut handle = cmd.spawn()?;
                drop(cmd);

                debug!("executing: {}", &command.script);
                std::io::copy(&mut reader, &mut bufw);

                let status = handle.wait()?;
                debug!("status of {}: {:?}", &command.script, status);
            }

            let response = CommandResponse {
                uuid,
                stream: None,
                task: None,
                log: req
                    .url()
                    .join(&format!("../../{}", log_file_path.display()))
                    .unwrap(),
            };
            Ok(Body::from_json(&response)?)
        }

        /*
         * GET /capabilities
         */
        pub async fn get_caps(_req: Request<()>) -> Result<Body, tide::Error> {
            let response = json!({
                "caps" : [
                    Git::has_capability(),
                    Cargo::has_capability(),
                ],
            });

            Ok(response.into())
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), tide::Error> {
    pretty_env_logger::init();
    dotenv().ok();
    let mut app = tide::new();

    #[cfg(not(debug_assertions))]
    {
        info!("Activating RELEASE mode configuration");
        app.with(driftwood::ApacheCombinedLogger);
    }

    /*
     * Create a logs directory if it doesn't exist
     */
    if !PathBuf::from(AGENT_LOGS_DIR).is_dir() {
        std::fs::create_dir(AGENT_LOGS_DIR).expect("Failed to create agent logs directory");
    }

    debug!("Configuring routes");
    app.at("/").get(routes::index);
    app.at("/agent-logs").serve_dir(AGENT_LOGS_DIR);
    routes::api::register(&mut app);
    app.listen("0.0.0.0:9000").await?;
    Ok(())
}
