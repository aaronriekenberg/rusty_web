extern crate iron;
#[macro_use] extern crate log;
extern crate logger;
extern crate router;
extern crate simple_logger;

use iron::Handler;
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

#[derive(Debug, Clone)]
struct CommandInfo {
  http_path: &'static str,
  command: &'static str,
  arguments: Vec<&'static str>,
  description: &'static str
}

fn get_commands() -> Vec<CommandInfo> {
  return vec![
    CommandInfo { http_path: "/ifconfig", command: "ifconfig",
                  arguments: vec![], description: "ifconfig" },
    CommandInfo { http_path: "/ls", command: "ls",
                  arguments: vec!["-l"], description: "ls -l" }
  ]
}

struct IndexHandler {
  index_string: String
}

impl IndexHandler {
  pub fn new(commands: &Vec<CommandInfo>) -> IndexHandler {
    let mut s = String::new();
    s.push_str("<html>");
    s.push_str("<head><title>Rusty Web</title></head>");
    s.push_str("<body>");
    s.push_str("<h1>Rusty Web</h1>");
    s.push_str("<h2>Commands:</h2>");
    s.push_str("<ul>");
    for command_info in commands.iter() {
      s.push_str("<li><a href=\"");
      s.push_str(command_info.http_path);
      s.push_str("\">");
      s.push_str(command_info.description);
      s.push_str("</a></li>");
    }
    s.push_str("</ul>");
    s.push_str("</body>");
    s.push_str("</html>");
    IndexHandler { index_string: s }
  }
}

impl Handler for IndexHandler {
  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    return Ok(Response::with((iron::status::Ok,
                              Header(ContentType::html()), 
                              self.index_string.clone())));
  }
}

struct CommandHandler {
  command_info: CommandInfo
}

impl Handler for CommandHandler {
  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    let mut command = Command::new(self.command_info.command);
    let mut args_string = String::new();
    for arg in self.command_info.arguments.iter() {
      command.arg(arg);
      args_string.push_str(arg);
      args_string.push(' ');
    }
    let command_output =
      match command.output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(err) => format!("command error: {}", err),
      };
    let mut s = String::new();
    s.push_str("<html>");
    s.push_str("<head><title>");
    s.push_str(&self.command_info.description);
    s.push_str("</title></head>");
    s.push_str("<body>");
    s.push_str("<pre>");
    s.push_str("$ ");
    s.push_str(&self.command_info.command);
    s.push(' ');
    s.push_str(&args_string);
    s.push_str("\n\n");
    s.push_str(&command_output);
    s.push_str("</pre>");
    s.push_str("</body>");
    s.push_str("</html>");
    return Ok(Response::with((iron::status::Ok, Header(ContentType::html()), s)));
  }
}

fn setup_router(router: &mut router::Router) {
  let commands = get_commands();

  router.get("/", IndexHandler::new(&commands), "index");

  for command_info in commands.iter() {
    let handler = CommandHandler { command_info: command_info.clone() };
    router.get(command_info.http_path, handler, command_info.description);
  }
}

fn main() {
  simple_logger::init_with_level(LogLevel::Info).unwrap();

  let mut router = Router::new();
  setup_router(&mut router);

  let mut chain = Chain::new(router);
  let (logger_before, logger_after) = Logger::new(None);
  chain.link_before(logger_before);
  chain.link_after(logger_after);

  match Iron::new(chain).http("localhost:3000") {
    Ok(listening) => info!("{:?}", listening),
    Err(err) => panic!("{:?}", err),
  }
}
