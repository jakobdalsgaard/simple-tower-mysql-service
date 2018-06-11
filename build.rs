extern crate tower_grpc_build;

fn main () {

  // build AccountInfoService
  tower_grpc_build::Config::new()
    .enable_server(true)
    .build(&["proto/services/simple_service.proto"], &["proto"])
    .unwrap_or_else(|e| panic!("protobuf compilation failed {}", e));

}
