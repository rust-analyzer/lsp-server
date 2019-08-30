//! A language server scaffold, exposing a synchronous crossbeam-channel based API.
//! This crate handles protocol handshaking and parsing messages, while you
//! control the message dispatch loop yourself.
//!
//! Run with `RUST_LOG=lsp_server=debug` to see all the messages.
mod msg;
mod stdio;
mod error;

use crossbeam_channel::{Receiver, Sender};

pub use crate::{
    error::{LspServerError, ProtocolError},
    msg::{ErrorCode, Message, Notification, Request, RequestId, Response, ResponseError},
    stdio::{stdio_transport, IoThreads},
};

/// Main entry point: runs the server from initialization to shutdown.
///
/// To attach server to standard input/output streams, use the `stdio_transport`
/// function to create corresponding `sender` and `receiver` pair.
///
/// `server` should use the `handle_shutdown` function to handle the `Shutdown`
/// request.
pub fn run_server<F, E>(
    caps: serde_json::Value,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    server: F,
) -> Result<(), LspServerError<E>>
where
    F: FnOnce(serde_json::Value, &Sender<Message>, &Receiver<Message>) -> Result<(), E>,
{
    log::info!("lsp server initializes");
    let params = initialize(&sender, &receiver, caps)?;
    log::info!("lsp server initialized, serving requests");
    server(params, &sender, &receiver).map_err(LspServerError::ServerError)?;
    log::info!("lsp server waiting for exit notification");
    match &receiver.recv() {
        Ok(Message::Notification(n)) if n.is_exit() => (),
        m => Err(ProtocolError(format!("unexpected message during shutdown: {:?}", m)))?,
    }
    log::info!("lsp server shutdown complete");
    Ok(())
}

/// If `req` is `Shutdown`, respond to it and return `true`, otherwise return `false`
pub fn handle_shutdown(req: &Request, sender: &Sender<Message>) -> bool {
    if !req.is_shutdown() {
        return false;
    }
    let resp = Response::new_ok(req.id.clone(), ());
    let _ = sender.send(resp.into());
    true
}

fn initialize(
    sender: &Sender<Message>,
    receiver: &Receiver<Message>,
    server_capabilities: serde_json::Value,
) -> Result<serde_json::Value, ProtocolError> {
    let (id, params) = match receiver.recv() {
        Ok(Message::Request(req)) => {
            if req.is_initialize() {
                (req.id, req.params)
            } else {
                return Err(ProtocolError(format!("expected initialize request, got {:?}", req)))?;
            }
        }
        msg => Err(ProtocolError(format!("expected initialize request, got {:?}", msg)))?,
    };
    let resp = Response::new_ok(
        id,
        serde_json::json!({
            "capabilities": server_capabilities,
        }),
    );
    sender.send(resp.into()).unwrap();
    match &receiver.recv() {
        Ok(Message::Notification(n)) if n.is_initialized() => (),
        _ => return Err(ProtocolError("expected initialized notification".to_string())),
    }
    Ok(params)
}
