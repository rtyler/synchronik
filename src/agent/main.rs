use dotenv::dotenv;
use log::*;

mod routes {
    use log::*;
    use tide::{Body, Request, StatusCode};

    /**
     *  GET /
     */
    pub async fn index(req: Request<()>) -> Result<Body, tide::Error> {
        Ok("Hello World from the Janky Agent".into())
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
    app.listen("0.0.0.0:9000").await?;
    Ok(())
}
