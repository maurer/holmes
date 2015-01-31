// This code heavily derived from capnp-rpc-rust's EzRpcServer
use std::str::FromStr;

use capnp_rpc::rpc_capnp::{message, return_};
use std::old_io::Acceptor;
use std::collections::hash_map::HashMap;
use capnp::{any_pointer, MessageBuilder, MallocMessageBuilder};
use capnp::capability::{ClientHook, FromClientHook, Server};
use capnp_rpc::rpc::{RpcConnectionState, RpcEvent, SturdyRefRestorer};
use capnp_rpc::capability::{LocalClient};
use std;
use std::sync::Arc;
use std::ops::Deref;
use std::thread::{Thread, JoinGuard};
use std::sync::atomic::{AtomicBool, Ordering};

enum ExportEvent {
    Restore(String, std::sync::mpsc::Sender<Option<Box<ClientHook+Send>>>),
    Register(String, Box<Server+Send>),
}

struct ExportedCaps {
    objects : HashMap<String, Box<ClientHook+Send>>,
}

impl ExportedCaps {
    pub fn new() -> std::sync::mpsc::Sender<ExportEvent> {
        let (chan, port) = std::sync::mpsc::channel::<ExportEvent>();

        std::thread::Thread::spawn(move || {
                let mut vat = ExportedCaps { objects : HashMap::new() };

                loop {
                    match port.recv() {
                        Ok(ExportEvent::Register(name, server)) => {
                            vat.objects.insert(name, Box::new(LocalClient::new(server)) as Box<ClientHook+Send>);
                        }
                        Ok(ExportEvent::Restore(name, return_chan)) => {
                            return_chan.send(Some(vat.objects[name].copy())).unwrap();
                        }
                        Err(_) => break,
                    }
                }
            });

        chan
    }
}

pub struct Restorer {
    sender : std::sync::mpsc::Sender<ExportEvent>,
}

impl Restorer {
    fn new(sender : std::sync::mpsc::Sender<ExportEvent>) -> Restorer {
        Restorer { sender : sender }
    }
}

impl SturdyRefRestorer for Restorer {
    fn restore(&self, obj_id : any_pointer::Reader) -> Option<Box<ClientHook+Send>> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.sender.send(ExportEvent::Restore(obj_id.get_as::<::capnp::text::Reader>().to_string(), tx)).unwrap();
        return rx.recv().unwrap();
    }
}

pub struct RpcServer {
  sender : std::sync::mpsc::Sender<ExportEvent>,
  tcp_acceptor : std::old_io::net::tcp::TcpAcceptor,
  pub shutdown : Arc<AtomicBool>
}

impl RpcServer {
  pub fn new(bind_address : &str) -> std::old_io::IoResult<RpcServer> {
    use std::old_io::net::{ip, tcp};
    use std::old_io::Listener;
    let addr : ip::SocketAddr = std::str::FromStr::from_str(bind_address).expect("bad bind address");
    let tcp_listener = try!(tcp::TcpListener::bind(addr));
    let tcp_acceptor = try!(tcp_listener.listen());
    let sender = ExportedCaps::new();
    Ok(RpcServer { sender : sender, tcp_acceptor : tcp_acceptor, shutdown : Arc::new(AtomicBool::new(false))})
  }
  pub fn export_cap(&self, name : &str, server : Box<::capnp::capability::Server+Send>) {
    self.sender.send(ExportEvent::Register(name.to_string(), server)).unwrap()
  }
  pub fn serve<'a>(self) -> JoinGuard<'a, ()> {
    std::thread::Thread::scoped(move || {
      use std::old_io::Acceptor;
      let mut server = self;
      let shutdown = server.shutdown.clone();
      for res in server.incoming() {
        println!("New connection!");
        if shutdown.load(Ordering::Acquire) {
          println!("Aborting!");
          break;
        }
        match res {
          Ok(()) => {}
          Err(e) => {
            println!("error: {}", e)
          }
        }
      }
    })
  }
}

impl std::old_io::Acceptor<()> for RpcServer {
  fn accept(&mut self) -> std::old_io::IoResult<()> {
    let sender2 = self.sender.clone();
    let tcp = try!(self.tcp_acceptor.accept());
    Thread::spawn(move || {
      let connection_state = RpcConnectionState::new();
      let _rpc_chan = connection_state.run(tcp.clone(), tcp, Restorer::new(sender2));
    });
    Ok(())
  }
}

// End derived code


