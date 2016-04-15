#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
extern crate env_logger;
extern crate iron;
extern crate router;
extern crate logger;
extern crate rustc_serialize;
extern crate mount;
extern crate rand;
extern crate persistent;
extern crate oven;
extern crate cookie;

pub mod routes;
pub mod result;
pub mod bind;
pub mod login;

use std::sync::Arc;

use iron::prelude::*;
use iron::status;
use router::Router;
use logger::Logger;
use logger::format::Format;
use mount::Mount;

use routes::{api, session};
use routes::session::{User, UserRepo};
use login::LoginManager;

static FORMAT: &'static str =
        "{method} {uri} -> {status} ({response-time} ms)";
   

fn hello(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "hello, world!")))
}

fn main() {
    env_logger::init().unwrap();
    let format = Format::new(FORMAT, vec![], vec![]);
    let (logger_before, logger_after) = Logger::new(Some(format.unwrap()));
    
    let cookie_signing_key = b"My secret key"[..].to_owned();
    
    let users = Arc::new(Box::new(session::InMemoryUserRepo::new()) as Box<session::UserRepo>);
    users.add_user(User::new("1".to_owned(), "admin".to_owned(), "admin".to_owned())).unwrap();
    let sessions = Arc::new(Box::new(session::InMemorySessions::new(users)) as Box<session::Sessions>);
    let login_manager = LoginManager::new(cookie_signing_key);
    let sessions_controller = session::SessionController::new(sessions, login_manager.clone());
    
    let mut router = Router::new();
    router.get("/hello", hello);
    router.get("/simple", api::simple);
    router.get("/session", bind::bind(session::get_session, sessions_controller.clone()));
    router.post("/session", bind::bind(session::post_session, sessions_controller.clone()));
    router.delete("/session", bind::bind(session::delete_session, sessions_controller.clone()));
    
    let mut mount = Mount::new();
    mount.mount("/api", router);
    
    let mut chain = Chain::new(mount);
    chain.around(login_manager);
    chain.link_before(logger_before);
    chain.link_after(logger_after);
    
    Iron::new(chain).http("0.0.0.0:8080").unwrap();
}
