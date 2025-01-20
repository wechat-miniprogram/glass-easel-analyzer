use context::{backend_configuration::BackendConfig, project::Project, ServerContext, ServerContextOptions};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response, ResponseError};

mod color;
mod completion;
mod context;
mod file;
mod folding;
mod hover;
mod logger;
mod reference;
mod semantic;
mod symbol;
mod utils;
mod wxml_utils;
mod wxss;
mod wxss_utils;

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
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp_types::CompletionOptions {
            resolve_provider: None,
            trigger_characters: Some(vec![String::from("<"), String::from("/"), String::from(" "), String::from("("), String::from("@"), String::from("#"), String::from(".")]),
            all_commit_characters: None,
            work_done_progress_options: lsp_types::WorkDoneProgressOptions { work_done_progress: None },
            completion_item: None,
        }),
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
        color_provider: Some(lsp_types::ColorProviderCapability::Simple(true)),
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
        workspace: Some(lsp_types::WorkspaceServerCapabilities {
            workspace_folders: Some(lsp_types::WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(lsp_types::OneOf::Left(true)),
            }),
            file_operations: None,
        }),
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
    handler!("shutdown", cleanup);
    handler!("textDocument/foldingRange", folding::folding_range);
    handler!("textDocument/semanticTokens/full", semantic::tokens_full);
    handler!("textDocument/semanticTokens/range", semantic::tokens_range);
    handler!("textDocument/definition", reference::find_definition);
    handler!("textDocument/declaration", reference::find_declaration);
    handler!("textDocument/references", reference::find_references);
    handler!("textDocument/documentSymbol", symbol::document_symbol);
    handler!("textDocument/hover", hover::hover);
    handler!("textDocument/completion", completion::completion);
    handler!("textDocument/documentColor", color::color);
    handler!("textDocument/colorPresentation", color::color_presentation);
    handler!("workspace/didChangeWorkspaceFolders", file::did_change_workspace_folders);

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
    async fn noop(_ctx: ServerContext, _params: serde_json::Value) -> anyhow::Result<()> { Ok(()) }

    // handlers for each method
    handler!("exit", noop);
    handler!("$/cancelRequest", noop);
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

fn generate_notification(method: impl Into<String>, params: impl serde::Serialize) -> Message {
    Message::Notification(Notification {
        method: method.into(),
        params: serde_json::to_value(params).unwrap(),
    })
}

async fn cleanup(ctx: ServerContext, _params: serde_json::Value) -> anyhow::Result<()> {
    ctx.clear_all_projects().await;
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
struct InitializeParams {
    #[serde(default)]
    initialization_options: InitializationOptions,
    capabilities: lsp_types::ClientCapabilities,
    work_done_token: lsp_types::ProgressToken,
}

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
struct InitializationOptions {
    #[serde(default)]
    backend_config: String,
    workspace_folders: Vec<String>,
    ignore_paths: Vec<String>,
}

async fn serve() -> anyhow::Result<()> {
    let (connection, _io_threads) = Connection::stdio();

    // handshake
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;
    let mut client_supported = true;
    if initialize_params.capabilities.workspace.as_ref().and_then(|x| x.workspace_folders) != Some(true) {
        client_supported = false;
    };
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
    connection.sender.send(generate_notification("$/progress", lsp_types::ProgressParams {
        token: initialize_params.work_done_token.clone(),
        value: lsp_types::ProgressParamsValue::WorkDone(
            lsp_types::WorkDoneProgress::Begin(lsp_types::WorkDoneProgressBegin {
                title: "Initializing glass-easel-analyzer".to_string(),
                message: Some("initializing".to_string()),
                ..Default::default()
            }),
        ),
    })).unwrap();

    // request workspace folders
    let mut projects = vec![];
    let ignore_paths: Vec<_> = initialize_params.initialization_options.ignore_paths.iter().map(|x| {
        std::path::PathBuf::from(x)
    }).collect();
    for uri in initialize_params.initialization_options.workspace_folders.iter() {
        let p_uri = lsp_types::Url::parse(uri).unwrap();
        let p = lsp_types::Url::to_file_path(&p_uri).unwrap_or_else(|_| {
            crate::utils::generate_non_fs_fake_path(&p_uri)
        });
        let name = p.file_name().and_then(|x| x.to_str()).unwrap_or_default();
        connection.sender.send(generate_notification("$/progress", lsp_types::ProgressParams {
            token: initialize_params.work_done_token.clone(),
            value: lsp_types::ProgressParamsValue::WorkDone(
                lsp_types::WorkDoneProgress::Report(lsp_types::WorkDoneProgressReport {
                    message: Some(format!("scanning components in {:?}", name)),
                    ..Default::default()
                }),
            ),
        })).unwrap();
        let found_projects = Project::search_projects(&p, &ignore_paths).await;
        if found_projects.len() == 0 {
            continue;
        }
        connection.sender.send(generate_notification("$/progress", lsp_types::ProgressParams {
            token: initialize_params.work_done_token.clone(),
            value: lsp_types::ProgressParamsValue::WorkDone(
                lsp_types::WorkDoneProgress::Report(lsp_types::WorkDoneProgressReport {
                    message: Some(format!("loading components in {}", name)),
                    ..Default::default()
                }),
            ),
        })).unwrap();
        for mut project in found_projects {
            project.init().await;
            projects.push(project);
        }
    }

    // send initialize done
    connection.sender.send(generate_notification("$/progress", lsp_types::ProgressParams {
        token: initialize_params.work_done_token.clone(),
        value: lsp_types::ProgressParamsValue::WorkDone(
            lsp_types::WorkDoneProgress::End(lsp_types::WorkDoneProgressEnd {
                message: Some("finished".to_string()),
                ..Default::default()
            }),
        ),
    })).unwrap();
    let initialize_result = lsp_types::InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(lsp_types::ServerInfo {
            name: String::from("glass-easel-analyzer"),
            version: None,
        }),
    };
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    // parse backend configuration
    let mut backend_config_failure = None;
    let has_backend_config = !initialize_params.initialization_options.backend_config.is_empty();
    let backend_config = if !has_backend_config {
        Default::default()
    } else {
        match toml::from_str(&initialize_params.initialization_options.backend_config) {
            Ok(x) => x,
            Err(err) => {
                backend_config_failure = Some(err);
                Default::default()
            }
        }
    };
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if !has_backend_config {
            log::warn!("Missing glass-easel backend configuration");
        } else if let Some(err) = backend_config_failure {
            log::error!("Failed to parse glass-easel backend configuration: {}", err);
        }
    });

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
    let server_context_options = ServerContextOptions { ignore_paths };
    let (server_context, sender) = {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let lsp_sender = lsp_sender.clone();
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                lsp_sender.send(msg).unwrap();
            }
        });
        let server_context = ServerContext::new(&sender, backend_config, projects, server_context_options);
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
