#[macro_use]
extern crate serde_json;

use std::path::PathBuf;

use async_std::channel::{bounded, Receiver, Sender};
use dotenv::dotenv;
use log::*;
use synchronik::CommandRequest;
use uuid::Uuid;

const AGENT_LOGS_DIR: &str = "agent-logs";

mod caps;

mod routes {
    use tide::{Body, Request};

    /**
     *  GET /
     */
    pub async fn index(_req: Request<crate::State>) -> Result<Body, tide::Error> {
        Ok("Hello World from the Synchronik Agent".into())
    }

    pub mod api {
        use crate::caps::*;
        use crate::*;
        use synchronik::{CommandRequest, CommandResponse};
        use tide::{Body, Request, Response, StatusCode};
        use uuid::Uuid;

        use std::path::Path;

        pub fn register(app: &mut tide::Server<State>) {
            app.at("/api/v1/capabilities").get(get_caps);
            app.at("/api/v1/execute").put(execute);
        }

        /*
         * PUT /execute
         *
         * This will take in the commands to actually execute
         */
        pub async fn execute(mut req: Request<State>) -> Result<Response, tide::Error> {
            // If we cannot accept work right now return an HTTP 409
            if req.state().channel.is_full() {
                let mut response = Response::new(StatusCode::Conflict);
                response.set_body("{}");
                return Ok(response);
            }

            let c: CommandRequest = req.body_json().await?;
            debug!("Commands to exec: {:?}", c);
            let uuid = Uuid::new_v4();
            // Create my log directory
            let log_dir = Path::new(AGENT_LOGS_DIR).join(uuid.hyphenated().to_string());
            // TODO: Handle this error
            std::fs::create_dir(log_dir.clone()).expect("Failed to create log dir");

            let log_file_path = log_dir.join("console.log");
            let work = Work {
                task: uuid,
                log_file: log_file_path.clone(),
                command: c,
            };
            req.state().channel.send(work).await?;

            let response = CommandResponse {
                uuid,
                stream: None,
                task: None,
                log: req
                    .url()
                    .join(&format!("../../{}", log_file_path.display()))
                    .unwrap(),
            };
            let mut http_response = Response::new(StatusCode::Created);
            http_response.set_body(Body::from_json(&response)?);
            Ok(http_response)
        }

        /*
         * GET /capabilities
         */
        pub async fn get_caps(_req: Request<State>) -> Result<Body, tide::Error> {
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

/*
 * Struct to encapsulate execution from a request handler to the worker thread
 */
#[derive(Clone, Debug)]
struct Work {
    task: Uuid,
    log_file: PathBuf,
    command: CommandRequest,
}

/*
 * State struct just carries data into Tide request handlers
 */
#[derive(Clone, Debug)]
pub struct State {
    channel: Sender<Work>,
}

/*
 * The worker function just does a busy loop executing Work
 */
async fn worker(receiver: Receiver<Work>) {
    debug!("Worker thread starting");

    while let Ok(work) = receiver.recv().await {
        let log_file = std::fs::File::create(&work.log_file).unwrap();
        let mut bufw = std::io::BufWriter::new(log_file);
        debug!(
            "Starting to execute the commands, output in {:?}",
            &work.log_file
        );
        for command in work.command.commands.iter() {
            debug!("Command: {:?}", command);
            use os_pipe::pipe;
            use std::process::Command;
            let mut cmd = Command::new("sh");
            cmd.args(["-xec", &command.script]);
            let (mut reader, writer) = pipe().expect("Failed to create pipe");
            let writer_clone = writer.try_clone().expect("Failed to clone writer pipe");
            cmd.stdout(writer);
            cmd.stderr(writer_clone);
            let mut handle = cmd.spawn().expect("Failed to launch command");
            drop(cmd);

            debug!("executing: {}", &command.script);
            std::io::copy(&mut reader, &mut bufw).expect("Failed to copy streams");

            let status = handle.wait().expect("Failed to wait on handle");
            debug!("status of {}: {:?}", &command.script, status);
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), tide::Error> {
    pretty_env_logger::init();
    dotenv().ok();
    let (sender, receiver) = bounded(1);
    async_std::task::spawn(worker(receiver));

    let state = State { channel: sender };
    let mut app = tide::with_state(state);

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
    app.at("/agent-logs").serve_dir(AGENT_LOGS_DIR)?;
    routes::api::register(&mut app);
    app.listen("0.0.0.0:9000").await?;
    Ok(())
}
