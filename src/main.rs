#[macro_use] extern crate log;
extern crate env_logger;
extern crate iron;
extern crate router;
extern crate logger;

use iron::prelude::*;
use iron::status;
use router::Router;
use logger::Logger;
use logger::format::Format;

static FORMAT: &'static str =
        "{method} {uri} -> {status} ({response-time} ms)";
   

fn hello(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "hello, world!")))
}

fn main() {
    env_logger::init().unwrap();
    let format = Format::new(FORMAT, vec![], vec![]);
    let (logger_before, logger_after) = Logger::new(Some(format.unwrap()));
    
    let mut router = Router::new();
    
    router.get("/", hello);
    
    let mut chain = Chain::new(router);
    
    chain.link_before(logger_before);
    chain.link_after(logger_after);
    
    Iron::new(chain).http("localhost:4000").unwrap();
}
