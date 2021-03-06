#![allow(deprecated)]

use ::error::Result;
use ::plugin_core::NewRpcRequest;
use mio::unix::UnixStream;
use mio;
use serde;
use std::path;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;

pub struct Client {
    event_loop_commands: mio::Sender<event_loop::Command>,
    rpc_loop_commands: rpc_loop::CommandSender,

    rpc_loop_thread: Option<thread::JoinHandle<()>>,
    event_loop_thread: Option<thread::JoinHandle<()>>,
}

impl Client {
    pub fn connect(socket_name: &path::Path) -> Result<Self> {
        let stream = try!(UnixStream::connect(&socket_name));

        let (commands_tx, commands_rx) = mpsc::channel();
        let (event_loop_thread, event_loop_commands) = event_loop::spawn(stream, commands_tx.clone());

        Ok(Client {
            event_loop_commands: event_loop_commands.clone(),
            rpc_loop_commands: commands_tx.clone(),
            rpc_loop_thread: Some(rpc_loop::spawn(commands_rx, commands_tx, event_loop_commands)),
            event_loop_thread: Some(event_loop_thread),
        })
    }

    pub fn new_rpc(&self, name: &str, rpc: Box<rpc::server::Rpc>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        let mut new_rpc = self.call("core.new_rpc", &NewRpcRequest {
            priority: rpc.priority(),
            name: name.into(),
        });
        let success = new_rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.rpc_loop_commands.send(rpc_loop::Command::NewRpc(name.into(), rpc)).expect("NewRpc");
    }

    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        rpc::client::Context::new(self.rpc_loop_commands.clone(), function, args).unwrap()
    }

    pub fn clone(&self) -> ThinClient {
        ThinClient {
            rpc_loop_commands: Mutex::new(self.rpc_loop_commands.clone()),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Either thread might have panicked at this point, so we can not rely on the sends to go
        // through. We just tell both (again) to Quit and hope they actually join. Order matters -
        // the event loop keeps everything going, so we should it down first.
        let _ = self.event_loop_commands.send(event_loop::Command::Quit);
        if let Some(thread) = self.event_loop_thread.take() {
            thread.join().expect("Joining event_loop_thread failed.");
        }

        let _ = self.rpc_loop_commands.send(rpc_loop::Command::Quit);
        if let Some(thread) = self.rpc_loop_thread.take() {
            thread.join().expect("Joining rpc_loop_thread failed.");
        }

    }
}

pub struct ThinClient {
    rpc_loop_commands: Mutex<rpc_loop::CommandSender>,
}

// NOCOM(#sirver): figure out the difference between a Sender, an Context and come up with better
// names.
impl ThinClient {
    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        let commands = {
            let commands = self.rpc_loop_commands.lock().unwrap();
            commands.clone()
        };
        rpc::client::Context::new(commands, function, args).unwrap()
    }

    pub fn clone(&self) -> Self {
        let commands = {
            let commands = self.rpc_loop_commands.lock().unwrap();
            commands.clone()
        };
        ThinClient {
            rpc_loop_commands: Mutex::new(commands),
        }
    }
}

mod event_loop;
mod rpc_loop;

pub mod rpc;
