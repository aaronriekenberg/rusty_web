extern crate iron;
#[macro_use]
extern crate log;
extern crate logger;
extern crate simple_logger;

use iron::prelude::Chain;
use iron::prelude::Iron;
use iron::prelude::Request;
use iron::prelude::Response;
use log::LogLevel;
use logger::Logger;

fn main() {
    simple_logger::init_with_level(LogLevel::Info).unwrap();

    let (logger_before, logger_after) = Logger::new(None);


    let handler = |_: &mut Request| {
        Ok(Response::with((iron::status::Ok, "Hello world!")))
    };

    let mut chain = Chain::new(handler);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    match Iron::new(chain).http("localhost:3000") {
        Ok(listening) => info!("{:?}", listening),
        Err(err) => panic!("{:?}", err),
    }
}
