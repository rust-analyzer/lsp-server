//! A minimal example LSP server that can only respond to the `gotoDefinition` request. To use
//! this example, execute it and then send an `initialize` request.
//!
//! ```no_run
//! Content-Length: 85
//!
//! {"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {"capabilities": {}}}
//! ```
//!
//! This will respond with a server response. Then send it a `initialized` notification which will
//! have no response.
//!
//! ```no_run
//! Content-Length: 59
//!
//! {"jsonrpc": "2.0", "method": "initialized", "params": {}}
//! ```
//!
//! Once these two are sent, then we enter the main loop of the server. The only request this
//! example can handle is `gotoDefinition`:
//!
//! ```no_run
//! Content-Length: 159
//!
//! {"jsonrpc": "2.0", "method": "textDocument/definition", "id": 2, "params": {"textDocument": {"uri": "file://temp"}, "position": {"line": 1, "character": 1}}}
//! ```
//!
//! To finish up without errors, send a shutdown request:
//!
//! ```no_run
//! Content-Length: 67
//!
//! {"jsonrpc": "2.0", "method": "shutdown", "id": 3, "params": null}
//! ```
//!
//! The server will exit the main loop and finally we send a `shutdown` notification to stop
//! the server.
//!
//! ```
//! Content-Length: 54
//!
//! {"jsonrpc": "2.0", "method": "exit", "params": null}
//! ```
use std::error::Error;

use log::info;
use lsp_types::{
    request::{GotoDefinition, GotoDefinitionResponse},
    InitializeParams, ServerCapabilities,
};

use lsp_server::{Connection, Message, Request, RequestId, Response};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Set up logging. Because `stdio_transport` gets a lock on stdout and stdin, we must have
    // our logging only write out to stderr.
    flexi_logger::Logger::with_str("info").start().unwrap();
    info!("starting generic LSP server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities::default()).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(&connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    info!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    info!("starting example main loop");
    for msg in &connection.receiver {
        info!("got msg: {:?}", msg);
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                info!("got request: {:?}", req);
                match cast::<GotoDefinition>(req) {
                    Ok((id, params)) => {
                        info!("got gotoDefinition request #{}: {:?}", id, params);
                        let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                        let result = serde_json::to_value(&result).unwrap();
                        let resp = Response { id, result: Some(result), error: None };
                        connection.sender.send(Message::Response(resp))?;
                        continue;
                    }
                    Err(req) => req,
                };
                // ...
            }
            Message::Response(resp) => {
                info!("got response: {:?}", resp);
            }
            Message::Notification(not) => {
                info!("got notification: {:?}", not);
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), Request>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
