//Derived heavily from ez_rpc.rs in capnp-rpc-rust
use capnp_rpc::rpc_capnp::{message, return_};

use capnp::{MessageBuilder, MallocMessageBuilder, ReaderOptions};
use capnp::private::capability::{ClientHook};
use capnp::capability::{FromClientHook, Server};
use capnp_rpc::rpc::{RpcConnectionState, RpcEvent};
use capnp_rpc::capability::{LocalClient};
use std::sync::mpsc::{channel, Receiver, Sender};

struct EmptyCap;

impl Server for EmptyCap {
    fn dispatch_call(&mut self, _interface_id : u64, _method_id : u16,
                     context : ::capnp::capability::CallContext<::capnp::any_pointer::Reader,
                                                                ::capnp::any_pointer::Builder>) {
        context.fail("Attempted to call a method on an empty capability.".to_string());
    }
}

#[derive(PartialEq, Clone, Eq, Debug)]
pub enum Command {
     Shutdown
}

#[derive(PartialEq, Clone, Eq, Debug)]
pub enum Status {
     Offline
}

pub struct RpcServer {
     tcp_listener : ::std::net::TcpListener,
     control      : Receiver<Command>,
     status       : Sender<Status>
}

impl RpcServer {
    pub fn new<A: ::std::net::ToSocketAddrs>(bind_address : A) -> ::std::io::Result<(RpcServer, Sender<Command>, Receiver<Status>)> {
        let tcp_listener = try!(::std::net::TcpListener::bind(bind_address));
        let (tx, rx) = channel();
        let (status_tx, status_rx) = channel();
        Ok((RpcServer { tcp_listener : tcp_listener, control : rx , status : status_tx}, tx, status_rx))
    }

    pub fn serve<'a>(self, bootstrap_interface : Box<Server + Send>) -> ::std::thread::JoinGuard<'a, ()> {
        ::std::thread::scoped(move || {
            let server = self;
            let bootstrap_interface = Box::new(LocalClient::new(bootstrap_interface));
            for stream_result in server.tcp_listener.incoming() {
                let bootstrap_interface = bootstrap_interface.copy();
                let tcp = stream_result.unwrap();
                match server.control.try_recv() {
                  Ok(Command::Shutdown) => {
                    server.status.send(Status::Offline);
                    break
                  }
                  _ => ()
                }
                ::std::thread::spawn(move || {
                    let connection_state = RpcConnectionState::new();
                    let _rpc_chan = connection_state.run(
                        tcp.try_clone().unwrap(),
                        tcp,
                        bootstrap_interface,
                        ReaderOptions::new());
                });
            }
        })
    }
}

