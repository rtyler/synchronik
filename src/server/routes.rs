/**
 * The routes module contains all the tide routes and the logic to fulfill the responses for each
 * route.
 *
 * Modules are nested for cleaner organization here
 */
use crate::AppState;
use tide::{Body, Request};

/**
 *  GET /
 */
pub async fn index(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
    let params = json!({
        "page": "home",
        "agents" : req.state().agents,
        "config" : req.state().config,
        "projects" : crate::dao::Project::list(&req.state().db).await?,
    });

    let mut body = req.state().render("index", &params).await?;
    body.set_mime("text/html");
    Ok(body)
}

/**
 * GET /project/:name
 */
pub async fn project(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
    let name: String = req.param("name")?.into();
    let params = json!({
        "name" : name,
    });

    let mut body = req.state().render("project", &params).await?;
    body.set_mime("text/html");
    Ok(body)
}

pub mod api {
    use crate::{AppState, JankyYml, Scm};
    use log::*;
    use tide::{Request, Response, StatusCode};

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
                            let _res = client
                                .put(
                                    agent
                                        .url
                                        .join("/api/v1/execute")
                                        .expect("Failed to join execute URL"),
                                )
                                .json(&commands)
                                .send()
                                .await?;

                            return Ok(
                                json!({ "msg": format!("Executing on {}", &agent.url) }).into()
                            );
                        }
                    }
                }
            }
            return Ok("{}".into());
        }
        Ok(Response::new(StatusCode::InternalServerError))
    }
}
