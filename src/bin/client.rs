extern crate futures;
extern crate http;
extern crate bytes;
extern crate env_logger;
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate tower_h2;
extern crate tower_http;
extern crate tower_grpc;

use futures::Future;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;
use std::env;

pub mod services {
    include!(concat!(env!("OUT_DIR"), "/services.rs"));
}

pub mod domain {
    include!(concat!(env!("OUT_DIR"), "/domain.rs"));
}

pub mod base {
    include!(concat!(env!("OUT_DIR"), "/base.rs"));
}

pub fn main() {
  let _ = ::env_logger::init();
  let mut core = Core::new().unwrap();
  let reactor = core.handle();

  // get cmd line arguments
  let args: Vec<String> = env::args().collect();
  let ident = &args[1];

  let addr = "[::1]:50051".parse().unwrap();
  let uri: http::Uri = format!("http://localhost:50055").parse().unwrap();

  let get_item_data = TcpStream::connect(&addr, &reactor)
    .and_then(move |socket| {
      Connection::handshake(socket, reactor).map_err(|_| panic!("failed HTTP/2.0 handshake"))
    })
    .map(move |conn| {
      use services::client::SimpleService;
      use tower_http::add_origin;

      let conn = add_origin::Builder::new().uri(uri).build(conn).unwrap();
      SimpleService::new(conn)
    })
    .and_then(|mut client| {
      use domain::ItemSpecifier;

      client.get_item_data(Request::new(ItemSpecifier { ident: ident.to_string() } )).map_err(|e| panic!("grpc request failed; err={:?}", e))
    })
    .and_then(|response| {
      println!("reponse is: {:?}", response);
      Ok(())
    })
    .map_err(|e| {
      println!("err = {:?}", e);
    });

    core.run(get_item_data).unwrap();
}

