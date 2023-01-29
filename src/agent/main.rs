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
        use tide::{Body, Request};

        pub fn register(app: &mut tide::Server<()>) {
            app.at("/api/v1/capabilities").get(get_caps);
        }

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
