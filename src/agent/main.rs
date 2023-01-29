#[macro_use]
extern crate serde_json;

use dotenv::dotenv;
use log::*;

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
        use janky::CommandRequest;
        use log::*;
        use tide::{Body, Request};

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

            for command in c.commands.iter() {
                use os_pipe::pipe;
                use std::io::{BufRead, BufReader};
                use std::process::Command;
                let mut cmd = Command::new("sh");
                cmd.args(["-c", &command.script]);
                let (reader, writer) = pipe().unwrap();
                let writer_clone = writer.try_clone().unwrap();
                cmd.stdout(writer);
                cmd.stderr(writer_clone);
                let mut handle = cmd.spawn()?;
                drop(cmd);

                debug!("executing: {}", &command.script);
                let bufr = BufReader::new(reader);
                for line in bufr.lines() {
                    if let Ok(buffer) = line {
                        debug!("output: {}", buffer);
                    }
                }

                let status = handle.wait()?;
                debug!("status of {}: {:?}", &command.script, status);
            }

            Ok("{}".into())
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

    debug!("Configuring routes");
    app.at("/").get(routes::index);
    routes::api::register(&mut app);
    app.listen("0.0.0.0:9000").await?;
    Ok(())
}
