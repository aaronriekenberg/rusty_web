extern crate chrono;
extern crate iron;
#[macro_use] extern crate log;
extern crate logger;
extern crate router;
#[macro_use] extern crate serde_derive;
extern crate serde_yaml;
extern crate simple_logger;

use chrono::prelude::Local;
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
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandInfo {
  http_path: String,
  description: String,
  command: String,
  args: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
  listen_address: String,
  main_page_title: String,
  commands: Vec<CommandInfo>
}

fn read_config() -> Result<Configuration, Box<Error>> {
  let mut file = File::open("config.yml")?;

  let mut file_contents = String::new();

  file.read_to_string(&mut file_contents)?;

  let configuration: Configuration = serde_yaml::from_str(&file_contents)?;

  return Ok(configuration);
}

fn current_time_string() -> String {
  let now = Local::now();  
  return now.format("%Y-%m-%d %H:%M:%S%.9f %z").to_string();
}

struct IndexHandler {
  index_string: String
}

impl IndexHandler {
  pub fn new(config: &Configuration) -> IndexHandler {
    let mut s = String::new();
    s.push_str("<html>");
    s.push_str("<head><title>");
    s.push_str(&config.main_page_title);
    s.push_str("</title></head>");
    s.push_str("<body>");
    s.push_str("<h1>");
    s.push_str(&config.main_page_title);
    s.push_str("</h1>");
    s.push_str("<h2>Commands:</h2>");
    s.push_str("<ul>");
    for command_info in config.commands.iter() {
      s.push_str("<li><a href=\"");
      s.push_str(&command_info.http_path);
      s.push_str("\">");
      s.push_str(&command_info.description);
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
    let mut command = Command::new(&self.command_info.command);
    let mut args_string = String::new();
    for arg in self.command_info.args.iter() {
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
    s.push_str("Now: ");
    s.push_str(&current_time_string());
    s.push_str("\n\n");
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

fn setup_router(
  router: &mut router::Router,
  config: &Configuration) {

  router.get("/", IndexHandler::new(config), "index");

  for command_info in config.commands.iter() {
    let handler = CommandHandler { command_info: command_info.clone() };
    router.get(&command_info.http_path, handler, &command_info.description);
  }
}

fn main() {
  simple_logger::init_with_level(LogLevel::Info).unwrap();

  let config = match read_config() {
    Ok(config) => config,
    Err(error) => panic!("error reading configuration: {}", error)
  };

  info!("config = {:?}", config);

  let mut router = Router::new();
  setup_router(&mut router, &config);

  let mut chain = Chain::new(router);
  let (logger_before, logger_after) = Logger::new(None);
  chain.link_before(logger_before);
  chain.link_after(logger_after);

  match Iron::new(chain).http(&config.listen_address) {
    Ok(listening) => info!("{:?}", listening),
    Err(err) => panic!("{:?}", err),
  }
}
