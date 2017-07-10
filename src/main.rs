extern crate chrono;
#[macro_use] extern crate horrorshow;
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
use horrorshow::helper::doctype;
use horrorshow::Template;
use iron::Handler;
use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::prelude::{Chain,Iron,IronResult,Request,Response};
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

#[derive(Debug, Serialize, Deserialize)]
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

    let s = html! {
      : doctype::HTML;
      html {
        head {
          title: &config.main_page_title;
          meta(name = "viewport", content = "width=device, initial-scale=1");
          link(rel = "stylesheet", type = "text/css", href = "style.css");
        }
        body {
          h2 {
            : &config.main_page_title;
          }
          h3 {
            : "Comamnds:"
          }
          ul {
            @ for command_info in &config.commands {
              li {
                a(href = &command_info.http_path) {
                  : &command_info.description;
                }
              }
            }
          }
          @ if static_paths_to_include.len() > 0 {
            h3 {
              : "Static Paths:";
            }
            ul {
              @ for static_path in &static_paths_to_include {
                li {
                  a(href = &static_path.http_path) {
                    : &static_path.fs_path;
                  }
                }
              }
            }
          }
        }
      }
    }.into_string().expect("error creating index html string");

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
  command_info: CommandInfo,
  args_string: String
}

impl CommandHandler {
  pub fn new(command_info: CommandInfo) -> CommandHandler {

    let mut args_string = String::new();

    for arg in &command_info.args {
      args_string.push_str(arg);
      args_string.push(' ');
    }

    CommandHandler { command_info: command_info, args_string: args_string }
  }
}

impl Handler for CommandHandler {
  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    let mut command = Command::new(&self.command_info.command);

    for arg in &self.command_info.args {
      command.arg(arg);
    }

    let command_output =
      match command.output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(err) => format!("command error: {}", err),
      };

    let s = html! {
      : doctype::HTML;
      html {
        head {
          title: &self.command_info.description;
          meta(name = "viewport", content = "width=device, initial-scale=1");
          link(rel = "stylesheet", type = "text/css", href = "style.css");
        }
        body {
          pre {
            : "Now: ";
            : &current_time_string();
            : "\n\n";
            : "$ ";
            : &self.command_info.command;
            : " ";
            : &self.args_string;
            : "\n\n";
            : &command_output;
          }
        }
      }
    }.into_string().unwrap_or_else(|err| format!("error executing template: {}", err));

    Ok(Response::with((iron::status::Ok, Header(ContentType::html()), s)))
  }
}

fn setup_router(
  router: &mut router::Router,
  config: &Configuration) {

  router.get("/", IndexHandler::new(config), "index");

  for command_info in &config.commands {
    let handler = CommandHandler::new(command_info.clone());
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
