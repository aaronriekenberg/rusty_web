extern crate iron;
#[macro_use] extern crate log;
extern crate logger;
extern crate router;
extern crate simple_logger;

use iron::prelude::Chain;
use iron::prelude::Iron;
use iron::prelude::IronResult;
use iron::prelude::Request;
use iron::prelude::Response;
use log::LogLevel;
use logger::Logger;
use router::Router;

fn handler(_: &mut Request) -> IronResult<Response> {
  Ok(Response::with((iron::status::Ok, "Hello world!")))
}

fn main() {
  simple_logger::init_with_level(LogLevel::Info).unwrap();

  let mut router = Router::new();
  router.get("/", handler, "index");

  let mut chain = Chain::new(router);
  let (logger_before, logger_after) = Logger::new(None);
  chain.link_before(logger_before);
  chain.link_after(logger_after);

  match Iron::new(chain).http("localhost:3000") {
    Ok(listening) => info!("{:?}", listening),
    Err(err) => panic!("{:?}", err),
  }
}
