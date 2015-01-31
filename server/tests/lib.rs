extern crate holmes;

use holmes::server_control::*;

#[test]
pub fn reboot() {
  println!("argblarg");
  let mut server =
      Server::new("127.0.0.1:8080",
                  DB::Postgres("postgresql://maurer@localhost/holmes_test"));
  println!("Booting server");
  unwrap(&server.boot());
  println!("Rebooting server");
  unwrap(&server.reboot());
  println!("Shutting server down");
  &server.shutdown();
  println!("Done!");  
}
