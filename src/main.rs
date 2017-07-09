extern crate chrono;
extern crate iron;
#[macro_use] extern crate log;
extern crate logger;
extern crate mount;
extern crate router;
#[macro_use] extern crate serde_derive;
extern crate serde_yaml;
extern crate simple_logger;
extern crate staticfile;

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
use mount::Mount;
use router::Router;
use staticfile::Static;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandInfo {
  http_path: String,
  description: String,
  command: String,
  args: Vec<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StaticPathInfo {
  http_path: String,
  fs_path: String,
  include_in_main_page: bool
}

#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
  listen_address: String,
  main_page_title: String,
  commands: Vec<CommandInfo>,
  static_paths: Vec<StaticPathInfo>
}

fn read_config(config_file: &str) -> Result<Configuration, Box<Error>> {
  info!("reading {}", config_file);

  let mut file = File::open(config_file)?;

  let mut file_contents = String::new();

  file.read_to_string(&mut file_contents)?;

  let configuration: Configuration = serde_yaml::from_str(&file_contents)?;

  Ok(configuration)
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
    let static_paths_to_include: Vec<_> = 
      config.static_paths.iter().filter(|s| s.include_in_main_page).collect();

    let mut s = String::new();
    s.push_str("<html>");
    s.push_str("<head>");
    s.push_str("<title>");
    s.push_str(&config.main_page_title);
    s.push_str("</title>");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    s.push_str("<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\">");
    s.push_str("</head>");
    s.push_str("<body>");
    s.push_str("<h1>");
    s.push_str(&config.main_page_title);
    s.push_str("</h1>");

    s.push_str("<h2>Commands:</h2>");
    s.push_str("<ul>");
    for command_info in &config.commands {
      s.push_str("<li><a href=\"");
      s.push_str(&command_info.http_path);
      s.push_str("\">");
      s.push_str(&command_info.description);
      s.push_str("</a></li>");
    }
    s.push_str("</ul>");

    if static_paths_to_include.len() > 0 {
      s.push_str("<h2>Static Paths:</h2>");
      s.push_str("<ul>");
      for static_path in &static_paths_to_include {
        s.push_str("<li><a href=\"");
        s.push_str(&static_path.http_path);
        s.push_str("\">");
        s.push_str(&static_path.fs_path);
        s.push_str("</a></li>");
      }
      s.push_str("</ul>");
    }

    s.push_str("</body>");
    s.push_str("</html>");

    IndexHandler { index_string: s }
  }
}

impl Handler for IndexHandler {
  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    Ok(Response::with((iron::status::Ok,
                       Header(ContentType::html()),
                       self.index_string.clone())))
  }
}

struct CommandHandler {
  command_info: CommandInfo
}

impl Handler for CommandHandler {
  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    let mut command = Command::new(&self.command_info.command);
    let mut args_string = String::new();
    for arg in &self.command_info.args {
      command.arg(arg);
      args_string.push_str(arg);
      args_string.push(' ');
    }

    let command_output =
      match command.output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(err) => format!("command error: {}", err),
      };

    let mut s = String::with_capacity(1024);
    s.push_str("<html>");
    s.push_str("<head>");
    s.push_str("<title>");
    s.push_str(&self.command_info.description);
    s.push_str("</title>");
    s.push_str("</head>");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    s.push_str("<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\">");
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
    s.push_str("\n");
    s.push_str("</pre>");
    s.push_str("</body>");
    s.push_str("</html>");

    Ok(Response::with((iron::status::Ok, Header(ContentType::html()), s)))
  }
}

fn setup_router(
  router: &mut router::Router,
  config: &Configuration) {

  router.get("/", IndexHandler::new(config), "index");

  for command_info in &config.commands {
    let handler = CommandHandler { command_info: command_info.clone() };
    router.get(&command_info.http_path, handler, &command_info.description);
  }
}

fn setup_mount(
  mount: &mut mount::Mount,
  router: router::Router,
  config: &Configuration) {

  mount.mount("/", router);

  for static_path_info in &config.static_paths {
    mount.mount(&static_path_info.http_path, Static::new(Path::new(&static_path_info.fs_path)));
  }
}

fn setup_chain(
  chain: &mut iron::prelude::Chain) {

  let (logger_before, logger_after) = Logger::new(None);

  chain.link_before(logger_before);
  chain.link_after(logger_after);
}

fn main() {
  simple_logger::init_with_level(LogLevel::Info).expect("init_with_level failed");

  let config_file = env::args().nth(1).expect("config file required as command line argument");

  let config = read_config(&config_file).expect("error reading configuration file");
  info!("config = {:?}", config);

  let mut router = Router::new();
  setup_router(&mut router, &config);

  let mut mount = Mount::new();
  setup_mount(&mut mount, router, &config);

  let mut chain = Chain::new(mount);
  setup_chain(&mut chain);

  let listening = Iron::new(chain).http(&config.listen_address).expect("error listening");
  info!("Listening on {}", listening.socket);
}
