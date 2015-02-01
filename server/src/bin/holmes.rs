extern crate holmes;

use holmes::server_control::*;

pub fn main () {
  let mut server =
    Server::new("127.0.0.1:8080",
                DB::Postgres("postgresql://maurer@localhost/holmes".to_string()));
  {unwrap(&server.boot());}
  &server.join();
}
