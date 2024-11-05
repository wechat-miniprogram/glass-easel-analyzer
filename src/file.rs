use lsp_types::{DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, FileChangeType, PublishDiagnosticsParams, TextDocumentContentChangeEvent};

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
        let diag = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                project.open_wxml(&abs_path, params.text_document.text)
            }
            Some("wxss") => {
                project.open_wxss(&abs_path, params.text_document.text)
            }
            Some("json") => {
                project.open_json(&abs_path, params.text_document.text)
            }
            _ => { return }
        };
        match diag {
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
    }).await);
    Ok(())
}

pub(crate) async fn did_change(ctx: ServerContext, params: DidChangeTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File changed: {}", params.text_document.uri.as_str());
    let uri = params.text_document.uri.clone();
    log_if_err(ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| {
        if let Some(content) = project.cached_file_content(&abs_path) {
            let new_content = apply_content_changes_to_content(&content.content, params.content_changes);
            let diag = match abs_path.extension().and_then(|x| x.to_str()) {
                Some("wxml") => {
                    project.open_wxml(&abs_path, new_content)
                }
                Some("wxss") => {
                    project.open_wxss(&abs_path, new_content)
                }
                Some("json") => {
                    project.open_json(&abs_path, new_content)
                }
                _ => { return }
            };
            match diag {
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
    }).await);
    Ok(())
}

pub(crate) async fn did_save(_ctx: ServerContext, params: DidSaveTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File saved: {}", params.text_document.uri.as_str());
    Ok(())
}

pub(crate) async fn did_close(ctx: ServerContext, params: DidCloseTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File closed: {}", params.text_document.uri.as_str());
    log_if_err(ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| {
        match abs_path.extension().and_then(|x| x.to_str()) {
            Some("json") => {
                log_if_err(project.close_json(&abs_path));
            }
            Some("wxml") => {
                log_if_err(project.close_wxml(&abs_path));
            }
            Some("wxss") => {
                log_if_err(project.close_wxml(&abs_path));
            }
            _ => {}
        }
    }).await);
    Ok(())
}

pub(crate) async fn did_change_watched_files(ctx: ServerContext, params: DidChangeWatchedFilesParams) -> anyhow::Result<()> {
    for change in params.changes {
        match change.typ {
            FileChangeType::CREATED | FileChangeType::CHANGED => {
                let _ = ctx.clone().project_thread_task(&change.uri, move |project, abs_path| {
                    project.file_changed(&abs_path);
                });
            }
            FileChangeType::DELETED => {
                let _ = ctx.clone().project_thread_task(&change.uri, move |project, abs_path| {
                    project.file_removed(&abs_path);
                });
            }
            _ => {}
        }
    }
    Ok(())
}
