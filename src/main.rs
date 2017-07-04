extern crate iron;
#[macro_use] extern crate log;
extern crate logger;
extern crate router;
extern crate simple_logger;

use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::prelude::Chain;
use iron::prelude::Iron;
use iron::prelude::IronResult;
use iron::prelude::Request;
use iron::prelude::Response;
use log::LogLevel;
use logger::Logger;
use router::Router;
use std::process::Command;

fn index_handler(_: &mut Request) -> IronResult<Response> {
  let mut s = String::new();
  s.push_str("<html>");
  s.push_str("<head><title>Rust is the new Ada</title></head>");
  s.push_str("<body>");
  s.push_str("<h1>Rust is the new Ada</h1>");
  s.push_str("<a href=\"ls\">ls</a>");
  s.push_str("</body>");
  s.push_str("</html>");
  return Ok(Response::with((iron::status::Ok, Header(ContentType::html()), s)));
}

fn ls_handler(_: &mut Request) -> IronResult<Response> {
  let command_output =
    match Command::new("ls").arg("-l").output() {
      Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
      Err(err) => format!("command error: {}", err),
    };
  let mut s = String::new();
  s.push_str("<html>");
  s.push_str("<head><title>ls</title></head>");
  s.push_str("<body>");
  s.push_str("<pre>");
  s.push_str("$ ls -l\n\n");
  s.push_str(&command_output);
  s.push_str("</pre>");
  s.push_str("</body>");
  s.push_str("</html>");
  return Ok(Response::with((iron::status::Ok, Header(ContentType::html()), s)));
}

fn main() {
  simple_logger::init_with_level(LogLevel::Info).unwrap();

  let mut router = Router::new();
  router.get("/", index_handler, "index");
  router.get("/ls", ls_handler, "ls");

  let mut chain = Chain::new(router);
  let (logger_before, logger_after) = Logger::new(None);
  chain.link_before(logger_before);
  chain.link_after(logger_after);

  match Iron::new(chain).http("localhost:3000") {
    Ok(listening) => info!("{:?}", listening),
    Err(err) => panic!("{:?}", err),
  }
}
