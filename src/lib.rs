//! A language server scaffold, exposing a synchronous crossbeam-channel based API.
//! This crate handles protocol handshaking and parsing messages, while you
//! control the message dispatch loop yourself.
//!
//! Run with `RUST_LOG=lsp_server=debug` to see all the messages.
mod msg;
mod stdio;
mod error;
mod socket;

use crossbeam_channel::{Receiver, Sender};
use std::net::ToSocketAddrs;

pub use crate::{
    error::ProtocolError,
    msg::{ErrorCode, Message, Notification, Request, RequestId, Response, ResponseError},
    stdio::IoThreads,
};

/// Connection is just a pair of channels of LSP messages.
pub struct Connection {
    pub sender: Sender<Message>,
    pub receiver: Receiver<Message>,
}

impl Connection {
    /// Create connection over standard in/standard out.
    ///
    /// Use this to create a real language server.
    pub fn stdio() -> (Connection, IoThreads) {
        let (sender, receiver, io_threads) = stdio::stdio_transport();
        (Connection { sender, receiver }, io_threads)
    }

    /// Create connection over standard in/sockets out.
    ///
    /// Use this to create a real language server.
    pub fn socket<A: ToSocketAddrs>(addr: A) -> (Connection, IoThreads) {
        let (sender, receiver, io_threads) = socket::socket_transport(addr);
        (Connection { sender, receiver }, io_threads)
    }

    /// Creates a pair of connected connections.
    ///
    /// Use this for testing.
    pub fn memory() -> (Connection, Connection) {
        let (s1, r1) = crossbeam_channel::unbounded();
        let (s2, r2) = crossbeam_channel::unbounded();
        (Connection { sender: s1, receiver: r2 }, Connection { sender: s2, receiver: r1 })
    }

    pub fn initialize(
        &self,
        server_capabilities: serde_json::Value,
    ) -> Result<serde_json::Value, ProtocolError> {
        let (id, params) = match self.receiver.recv() {
            Ok(Message::Request(req)) => {
                if req.is_initialize() {
                    (req.id, req.params)
                } else {
                    return Err(ProtocolError(format!(
                        "expected initialize request, got {:?}",
                        req
                    )));
                }
            }
            msg => {
                return Err(ProtocolError(format!("expected initialize request, got {:?}", msg)))
            }
        };
        let resp = Response::new_ok(
            id,
            serde_json::json!({
                "capabilities": server_capabilities,
            }),
        );
        self.sender.send(resp.into()).unwrap();
        match &self.receiver.recv() {
            Ok(Message::Notification(n)) if n.is_initialized() => (),
            _ => return Err(ProtocolError("expected initialized notification".to_string())),
        }
        Ok(params)
    }

    /// If `req` is `Shutdown`, respond to it and return `true`, otherwise return `false`
    pub fn handle_shutdown(&self, req: &Request) -> Result<bool, ProtocolError> {
        if !req.is_shutdown() {
            return Ok(false);
        }
        let resp = Response::new_ok(req.id.clone(), ());
        let _ = self.sender.send(resp.into());
        match &self.receiver.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Message::Notification(n)) if n.is_exit() => (),
            m => return Err(ProtocolError(format!("unexpected message during shutdown: {:?}", m))),
        }
        Ok(true)
    }
}
