extern crate env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate prost_derive;
extern crate futures;
extern crate tokio_core;
extern crate http;
extern crate tower_h2;
extern crate tower_grpc;
#[macro_use]
extern crate mysql_async;


use std::env;
use std::process;

use futures::{Future, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};
use tower_h2::Server;
use tower_grpc::{Request, Response};
use mysql_async::prelude::*;
use mysql_async::time::Timespec;
use mysql_async::from_value;

use std::collections::HashMap;
use std::collections::LinkedList;

pub mod services {
    include!(concat!(env!("OUT_DIR"), "/services.rs"));
}

pub mod domain {
    include!(concat!(env!("OUT_DIR"), "/domain.rs"));
}

pub mod base {
    include!(concat!(env!("OUT_DIR"), "/base.rs"));
}

use services::server;

#[derive(Clone, Debug)]
struct ItemServer {
    item_data_cache: HashMap<String, domain::ItemData>,
    item_data_cache_order: LinkedList<String>,
    item_data_cache_size: isize,
    item_data_cache_size_max: isize,
    mysql_pool: mysql_async::Pool,
}

impl ItemServer {

    fn new (max_cache_size: isize, mysql_url: String, reactor_handle: &Handle) -> ItemServer {
        let pool = mysql_async::Pool::new(mysql_url, reactor_handle);
        ItemServer {
            item_data_cache: HashMap::new(),
            item_data_cache_order: LinkedList::new(),
            item_data_cache_size: 0,
            item_data_cache_size_max: max_cache_size,
            mysql_pool: pool,
        }
    }

    fn retrieve_item_data (&mut self, ident: String) -> impl Future<Item = Response<domain::ItemData>, Error = tower_grpc::Error> {
  
      let sql_query = r#"SELECT id, ident, name, created_at, description
FROM items
WHERE ident=:ident"#;

      // get the mysql connection
      let connection = self.mysql_pool.get_conn();

      // take the connection and execute query
      let query = connection.and_then(move |conn| conn.prep_exec(sql_query, params! { ident }));

      // get the result set and reduce it
      // actually no need to reduce it, we can merely err on more than one row.
      let result = query.and_then(|result|
        result.reduce_and_drop(
                         None,
                         |_lastval, mut row| {
                           Some(domain::ItemData {
                               // assume all are non-nulls.... gotta look into this later
                               id: from_value(row.take(0).unwrap()),
                               ident: from_value(row.take(1).unwrap()), 
                               name: from_value(row.take(2).unwrap()), 
                               created_at: mysql_value_to_timestamp(row.take(3)),
                               description: from_value(row.take(6).unwrap()),
                           })
                       })).map_err(|e| { eprintln!("Error executing the query: {}", e); make_grpc_error(tower_grpc::Status::INTERNAL) } );

      // map into futures
      result.then(|result| match result {

        // we found data, return it
        Ok((_, Some(object))) => { 
          println!("object is: {:?}", object);
          self.item_data_cache.insert(ident, object);
          futures::future::ok(Response::new(object))
        },

        // none found, return error
        Ok((_, None)) => futures::future::err(make_grpc_error(tower_grpc::Status::NOT_FOUND)),

        // darn, got an error (it's a grpc error), send it
        Err(e) => futures::future::err(e),
      })
 }
}


fn make_grpc_error (status: tower_grpc::Status) -> tower_grpc::Error {
    tower_grpc::Error::Grpc(status, http::HeaderMap::new())
}

fn mysql_value_to_timestamp (valuep: Option<mysql_async::Value>) -> (Option<base::Timestamp>) {
   match valuep {
     Some(mysql_async::Value::NULL) => None,
     Some(value) => {
        let ts: Timespec = from_value(value);
        Some(base::Timestamp {
          seconds: ts.sec as u64,
          nanos: ts.nsec as u32,
        }) },
     _ => None,
   }
}

impl server::SimpleService for ItemServer {
    type GetItemDataFuture = Box<Future<Item=Response<domain::ItemData>, Error = tower_grpc::Error>>;
   
    fn get_item_data (&mut self, request: Request<domain::ItemSpecifier>) -> Self::GetItemDataFuture {

        let ident = request.into_inner().ident;

        if self.item_data_cache.contains_key(&ident) {
          let cached = self.item_data_cache.get(&ident);
          Box::new(futures::future::ok(Response::new(cached.unwrap().clone())))
        } else {
          let future = self.retrieve_item_data (ident);
          Box::new(future)
        }

    }

}
   

fn main() {


    let mysql_url = env::var("MYSQL_URL").unwrap_or_else(|_| {
      println!("Required environment parameter MYSQL_URL not found, exiting");
      process::exit(1);
    });
    
    let max_cache_size = env::var("MAX_CACHE_SIZE").unwrap_or_else(|_| {
      println!("Required environment parameter MAX_CACHE_SIZE not found, exiting");
      process::exit(1);
    });

    let cache_size = max_cache_size.parse::<isize>().unwrap_or_else(|_| {
      println!("Environment parameter MAX_CACHE_SIZE cannot be parsed as integer, was: {:?}", max_cache_size);
      process::exit(1);
    });

    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    // Create an Item server of given size, connecting to given mysql and using given reactor
    let new_service = server::SimpleServiceServer::new(ItemServer::new(cache_size, mysql_url, &reactor));

    let h2 = Server::new (new_service, Default::default(), reactor.clone());

    let addr = "[::1]:50051".parse().unwrap();
    let bind = TcpListener::bind(&addr, &reactor).expect("bind");

    let serve = bind.incoming()
        .fold((h2, reactor), |(h2, reactor), (sock, _)| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = h2.serve(sock);
            reactor.spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok((h2, reactor))
        });

    core.run(serve).unwrap();
}

