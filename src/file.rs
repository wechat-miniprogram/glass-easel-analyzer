use lsp_types::{DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, PublishDiagnosticsParams, TextDocumentContentChangeEvent};

use crate::{utils::log_if_err, ServerContext};

fn apply_content_changes_to_content(content: &str, changes: Vec<TextDocumentContentChangeEvent>) -> String {
    if changes.len() == 0 {
        return content.to_string();
    }
    let mut ret = String::new();
    for change in changes {
        if let Some(_range) = change.range {
            todo!()
        } else {
            ret = change.text;
        }
    }
    ret
}

pub(crate) async fn did_open(ctx: ServerContext, params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File opened: {}", params.text_document.uri.as_str());
    let uri = params.text_document.uri.clone();
    log_if_err(ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| {
        match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                match project.set_wxml(abs_path, params.text_document.text) {
                    Ok(diagnostics) => {
                        log_if_err(ctx.send_notification("textDocument/publishDiagnostics", PublishDiagnosticsParams {
                            uri,
                            diagnostics,
                            version: None,
                        }));
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
            _ => {}
        }
    }).await);
    Ok(())
}

pub(crate) async fn did_change(ctx: ServerContext, params: DidChangeTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File changed: {}", params.text_document.uri.as_str());
    let uri = params.text_document.uri.clone();
    log_if_err(ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| {
        match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                if let Some(content) = project.get_file_content(&abs_path) {
                    let new_content = apply_content_changes_to_content(content, params.content_changes);
                    match project.set_wxml(abs_path, new_content) {
                        Ok(diagnostics) => {
                            log_if_err(ctx.send_notification("textDocument/publishDiagnostics", PublishDiagnosticsParams {
                                uri,
                                diagnostics,
                                version: None,
                            }));
                        }
                        Err(err) => {
                            log::error!("{}", err);
                        }
                    }
                } else {
                    log::warn!("The LSP client tried to update a non-opened file");
                }
            }
            _ => {}
        }
    }).await);
    Ok(())
}

pub(crate) async fn did_save(_ctx: ServerContext, params: DidSaveTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File saved: {}", params.text_document.uri.as_str());
    Ok(())
}

pub(crate) async fn did_close(ctx: ServerContext, params: DidCloseTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File saved: {}", params.text_document.uri.as_str());
    log_if_err(ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| {
        match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                log_if_err(project.remove_wxml(abs_path));
            }
            _ => {}
        }
    }).await);
    Ok(())
}
