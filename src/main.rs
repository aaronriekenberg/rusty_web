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

  pub fn new(config: &Configuration) -> Result<IndexHandler, Box<Error>> {

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
    }.into_string()?;

    Ok(IndexHandler { index_string: s })
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
    let mut first: bool = true;

    for arg in &command_info.args {
      if !first {
        args_string.push(' ');
      }
      args_string.push_str(arg);
      first = false;
    }

    CommandHandler { command_info: command_info, args_string: args_string }
  }

  fn run_command(&self) -> String {

    let mut command = Command::new(&self.command_info.command);

    for arg in &self.command_info.args {
      command.arg(arg);
    }

    let command_output =
      match command.output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(err) => format!("command error: {}", err),
      };

    command_output
  }

  fn build_pre_string(&self, command_output: String) -> String {

    let mut pre_string = String::with_capacity(command_output.len() + 100);

    pre_string.push_str("Now: ");
    pre_string.push_str(&current_time_string());
    pre_string.push_str("\n\n");
    pre_string.push_str("$ ");
    pre_string.push_str(&self.command_info.command);
    if self.args_string.len() > 0 {
      pre_string.push_str(" ");
      pre_string.push_str(&self.args_string);
    }
    pre_string.push_str("\n\n");
    pre_string.push_str(&command_output);

    pre_string
  }

  fn build_html_string(&self, pre_string: String) -> String {

    let html_string = html! {
      : doctype::HTML;
      html {
        head {
          title: &self.command_info.description;
          meta(name = "viewport", content = "width=device, initial-scale=1");
          link(rel = "stylesheet", type = "text/css", href = "style.css");
        }
        body {
          pre {
            : pre_string
          }
        }
      }
    }.into_string()
     .unwrap_or_else(|err| format!("error executing template: {}", err));

    html_string
  }

}

impl Handler for CommandHandler {

  fn handle(&self, _: &mut Request) -> IronResult<Response> {
    let command_output = self.run_command();

    let pre_string = self.build_pre_string(command_output);

    let html_string = self.build_html_string(pre_string);

    Ok(Response::with((iron::status::Ok, Header(ContentType::html()), html_string)))
  }

}

fn setup_router(
  router: &mut router::Router,
  config: &Configuration) {

  let index_handler = IndexHandler::new(config).expect("error creating IndexHandler");
  router.get("/", index_handler, "index");

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
