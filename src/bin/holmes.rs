extern crate holmes;
extern crate getopts;

use getopts::Options;
use std::env;

use holmes::server_control::*;

pub fn main () {
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();

  let mut opts = Options::new();
  let default_addr = "127.0.0.1:8080";
  opts.optopt("-a", "address", "address to listen on", default_addr);
  opts.optflag("h", "help", "print this help menu");
  opts.optflag("", "dump-capnp", "dump the protocol definition file");
  let matches = match opts.parse(&args[1..]) {
      Ok(m) => { m }
      Err(f) => { panic!(f.to_string()) }
  };
  if matches.opt_present("dump-capnp") {
    print!("{}", include_str!("../holmes.capnp"));
    return;
  };
  if matches.opt_present("help") {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
    return;
  };
  let addr = &matches.opt_str("address").unwrap_or(default_addr.to_string());
  let mut server =
    Server::new(addr,
                DB::Postgres("postgresql://localhost/holmes".to_string()));
  {&server.boot().unwrap();}
  &server.join();
}
