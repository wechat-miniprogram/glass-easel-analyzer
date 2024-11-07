use context::ServerContext;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response, ResponseError};

mod context;
mod file;
mod folding;
mod logger;
mod reference;
mod semantic;
mod symbol;
pub(crate) mod utils;
pub(crate) mod wxml_utils;

fn server_capabilities() -> lsp_types::ServerCapabilities {
    lsp_types::ServerCapabilities {
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp_types::TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
            },
        )),
        // hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        // completion_provider: Some(lsp_types::CompletionOptions {
        //     resolve_provider: None,
        //     trigger_characters: Some(vec![String::from("<")]),
        //     all_commit_characters: None,
        //     work_done_progress_options: lsp_types::WorkDoneProgressOptions { work_done_progress: None },
        //     completion_item: None,
        // }),
        // signature_help_provider: Some(lsp_types::SignatureHelpOptions {
        //     trigger_characters: None,
        //     retrigger_characters: None,
        //     work_done_progress_options: lsp_types::WorkDoneProgressOptions { work_done_progress: None },
        // }),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        declaration_provider: Some(lsp_types::DeclarationCapability::Simple(true)),
        // type_definition_provider: Some(lsp_types::TypeDefinitionProviderCapability::Simple(true)),
        // implementation_provider: Some(lsp_types::ImplementationProviderCapability::Simple(true)),
        references_provider: Some(lsp_types::OneOf::Left(true)),
        // document_highlight_provider: Some(lsp_types::OneOf::Left(true)),
        document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
        folding_range_provider: Some(lsp_types::FoldingRangeProviderCapability::Simple(true)),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(lsp_types::SemanticTokensOptions {
                work_done_progress_options: lsp_types::WorkDoneProgressOptions { work_done_progress: None },
                legend: lsp_types::SemanticTokensLegend {
                    token_types: semantic::TOKEN_TYPES.into(),
                    token_modifiers: semantic::TOKEN_MODIFIERS.into(),
                },
                range: Some(true),
                full: Some(lsp_types::SemanticTokensFullOptions::Delta { delta: Some(false) }),
            })
        ),
        ..Default::default()
    }
}

async fn handle_request(ctx: ServerContext, Request { id, method, params }: Request) -> anyhow::Result<Response> {
    macro_rules! handler {
        ($name:expr, $f:path) => {
            if method.as_str() == $name {
                let params = serde_json::from_value(params)
                    .map_err(|err| anyhow::Error::from(err).context(format!("Invalid params on {:?}", method)))?;
                let ret = $f(ctx, params).await?;
                let res = Response { id, result: Some(serde_json::to_value(ret)?), error: None };
                return Ok(res);
            }
        };
    }

    // handlers for each method
    handler!("textDocument/foldingRange", folding::folding_range);
    handler!("textDocument/semanticTokens/full", semantic::tokens_full);
    handler!("textDocument/semanticTokens/range", semantic::tokens_range);
    handler!("textDocument/definition", reference::find_definition);
    handler!("textDocument/declaration", reference::find_declaration);
    handler!("textDocument/references", reference::find_references);
    handler!("textDocument/documentSymbol", symbol::document_symbol);

    // method not found
    log::warn!("Missing LSP request handler for {:?}", method);
    let message = format!("Cannot find LSP method {:?}", method);
    let err = ResponseError { code: ErrorCode::MethodNotFound as i32, message, data: None };
    Ok(Response { id, result: None, error: Some(err) })
}

async fn handle_notification(ctx: ServerContext, Notification { method, params }: Notification) -> anyhow::Result<()> {
    macro_rules! handler {
        ($name:expr, $f:path) => {
            if method.as_str() == $name {
                let params = serde_json::from_value(params)
                    .map_err(|err| anyhow::Error::from(err).context(format!("Invalid params on {:?}", method)))?;
                return Ok($f(ctx, params).await?);
            }
        };
    }

    // handlers for each method
    handler!("$/setTrace", logger::set_trace);
    handler!("textDocument/didOpen", file::did_open);
    handler!("textDocument/didChange", file::did_change);
    handler!("textDocument/didSave", file::did_save);
    handler!("textDocument/didClose", file::did_close);
    handler!("workspace/didChangeWatchedFiles", file::did_change_watched_files);

    // method not found
    log::warn!("Missing LSP notification handler for {:?}", method);
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams {
    #[serde(default)]
    #[allow(dead_code)]
    backend_configuration: String,
    capabilities: lsp_types::ClientCapabilities,
}

async fn serve() -> anyhow::Result<()> {
    let (connection, _io_threads) = Connection::stdio();

    // handshake
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;
    let mut client_supported = true;
    if initialize_params.capabilities.workspace.as_ref().and_then(|x| x.did_change_watched_files.and_then(|x| x.dynamic_registration)) != Some(true) {
        client_supported = false;
    };
    if initialize_params.capabilities.text_document.as_ref().and_then(|x| x.definition.and_then(|x| x.link_support)) != Some(true) {
        client_supported = false;
    };
    if initialize_params.capabilities.text_document.as_ref().and_then(|x| x.declaration.and_then(|x| x.link_support)) != Some(true) {
        client_supported = false;
    };
    if !client_supported {
        log::error!("The client does not have enough LSP capabilities");
        return Err(anyhow::Error::msg("unsupported client"));
    }
    let initialize_result = lsp_types::InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(lsp_types::ServerInfo {
            name: String::from("glass-easel-analyzer"),
            version: None,
        }),
    };
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    // register capabilities
    let registrations = lsp_types::RegistrationParams {
        registrations: vec![
            lsp_types::Registration {
                id: "workspace/didChangeWatchedFiles".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(serde_json::to_value(lsp_types::DidChangeWatchedFilesRegistrationOptions {
                    watchers: vec![lsp_types::FileSystemWatcher {
                        glob_pattern: lsp_types::GlobPattern::String("**/*.{json,wxml,wxss}".to_string()),
                        kind: Some(lsp_types::WatchKind::all()),
                    }],
                })?),
            },
        ],
    };
    connection.sender.send(Message::Request(Request {
        id: "client/registerCapability".to_string().into(),
        method: "client/registerCapability".to_string(),
        params: serde_json::to_value(registrations)?,
    }))?;

    // generate a `ServerContext`
    let Connection { sender: lsp_sender, receiver: lsp_receiver } = connection;
    let (server_context, sender) = {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let lsp_sender = lsp_sender.clone();
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                lsp_sender.send(msg).unwrap();
            }
        });
        let server_context = ServerContext::new(&sender);
        (server_context, sender)
    };
    logger::set_trace(server_context.clone(), lsp_types::SetTraceParams { value: lsp_types::TraceValue::Off }).await?;

    // waiting requests on a separate thread
    let connection_thread = {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn_local(async move {
            while let Some(msg) = receiver.recv().await {
                let ctx = server_context.clone();
                match msg {
                    Message::Request(req) => {
                        match handle_request(ctx, req).await {
                            Ok(res) => {
                                if let Err(err) = lsp_sender.send(Message::Response(res)) {
                                    log::error!("{}", err);
                                }
                            }
                            Err(err) => {
                                log::error!("{}", err);
                            }
                        }
                    }
                    Message::Response(Response { id: _, result: _, error }) => {
                        if let Some(err) = error {
                            log::error!("LSP response error: {:?}", err.message);
                        }
                        // log::warn!("Missing LSP response handler for {:?}", id);
                    }
                    Message::Notification(note) => {
                        if let Err(err) = handle_notification(ctx, note).await {
                            log::error!("{}", err);
                        }
                    }
                }
            }
        });
        tokio::task::spawn_blocking(move || {
            while let Ok(msg) = lsp_receiver.recv() {
                sender.send(msg).unwrap();
            }
        })
    };
    connection_thread.await?;
    std::mem::drop(sender);

    Ok(())
}

pub async fn run() -> anyhow::Result<()> {
    logger::init_trace();
    let local = tokio::task::LocalSet::new();
    local.run_until(serve()).await
}
