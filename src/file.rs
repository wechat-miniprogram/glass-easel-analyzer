use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileChangeType, PublishDiagnosticsParams, TextDocumentContentChangeEvent,
};

use crate::{context::{project::Project, FileLang}, utils::log_if_err, ServerContext};

fn apply_content_changes_to_content(
    content: &str,
    changes: Vec<TextDocumentContentChangeEvent>,
) -> String {
    if changes.len() == 0 {
        return content.to_string();
    }
    let mut ret = String::new();
    for change in changes {
        if let Some(_range) = change.range {
            todo!() // TODO support range update
        } else {
            ret = change.text;
        }
    }
    ret
}

pub(crate) async fn did_open(
    ctx: ServerContext,
    params: DidOpenTextDocumentParams,
) -> anyhow::Result<()> {
    log::debug!("File opened: {}", params.text_document.uri.as_str());
    let uri = params.text_document.uri.clone();
    log_if_err(
        ctx.clone()
            .project_thread_task(&params.text_document.uri, move |project, abs_path, _| {
                let diag = match params.text_document.language_id.as_str() {
                    "wxml" => project.open_wxml(&abs_path, params.text_document.text),
                    "wxss" => project.open_wxss(&abs_path, params.text_document.text),
                    "json" => project.open_json(&abs_path, params.text_document.text),
                    _ => return,
                };
                match diag {
                    Ok(diagnostics) => {
                        log_if_err(ctx.send_notification(
                            "textDocument/publishDiagnostics",
                            PublishDiagnosticsParams {
                                uri,
                                diagnostics,
                                version: None,
                            },
                        ));
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            })
            .await,
    );
    Ok(())
}

pub(crate) async fn did_change(
    ctx: ServerContext,
    params: DidChangeTextDocumentParams,
) -> anyhow::Result<()> {
    log::debug!("File changed: {}", params.text_document.uri.as_str());
    let uri = params.text_document.uri.clone();
    log_if_err(
        ctx.clone()
            .project_thread_task(&params.text_document.uri, move |project, abs_path, file_lang| {
                if let Some(content) = project.cached_file_content(&abs_path) {
                    let new_content =
                        apply_content_changes_to_content(&content.content, params.content_changes);
                    let diag = match file_lang {
                        FileLang::Wxml => project.open_wxml(&abs_path, new_content),
                        FileLang::Wxss => project.open_wxss(&abs_path, new_content),
                        FileLang::Json => project.open_json(&abs_path, new_content),
                        _ => return,
                    };
                    match diag {
                        Ok(diagnostics) => {
                            log_if_err(ctx.send_notification(
                                "textDocument/publishDiagnostics",
                                PublishDiagnosticsParams {
                                    uri,
                                    diagnostics,
                                    version: None,
                                },
                            ));
                        }
                        Err(err) => {
                            log::error!("{}", err);
                        }
                    }
                }
            })
            .await,
    );
    Ok(())
}

pub(crate) async fn did_save(
    _ctx: ServerContext,
    params: DidSaveTextDocumentParams,
) -> anyhow::Result<()> {
    log::debug!("File saved: {}", params.text_document.uri.as_str());
    Ok(())
}

pub(crate) async fn did_close(
    ctx: ServerContext,
    params: DidCloseTextDocumentParams,
) -> anyhow::Result<()> {
    log::debug!("File closed: {}", params.text_document.uri.as_str());
    log_if_err(
        ctx.clone()
            .project_thread_task(
                &params.text_document.uri,
                move |project, abs_path, file_lang| match file_lang {
                    FileLang::Json => {
                        log_if_err(project.close_json(&abs_path));
                    }
                    FileLang::Wxml => {
                        log_if_err(project.close_wxml(&abs_path));
                    }
                    FileLang::Wxss => {
                        log_if_err(project.close_wxss(&abs_path));
                    }
                    _ => {}
                },
            )
            .await,
    );
    Ok(())
}

pub(crate) async fn did_change_watched_files(
    ctx: ServerContext,
    params: DidChangeWatchedFilesParams,
) -> anyhow::Result<()> {
    for change in params.changes {
        match change.typ {
            FileChangeType::CREATED | FileChangeType::CHANGED => {
                log_if_err(
                    ctx.clone()
                        .project_thread_task(&change.uri, move |project, abs_path, _| {
                            project.file_created_or_changed(&abs_path);
                        })
                        .await,
                );
            }
            FileChangeType::DELETED => {
                log_if_err(
                    ctx.clone()
                        .project_thread_task(&change.uri, move |project, abs_path, _| {
                            project.file_removed(&abs_path);
                        })
                        .await,
                );
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) async fn did_change_workspace_folders(
    mut ctx: ServerContext,
    params: DidChangeWorkspaceFoldersParams,
) -> anyhow::Result<()> {
    for folder in params.event.added.iter() {
        let p = lsp_types::Url::to_file_path(&folder.uri)
            .unwrap_or_else(|_| crate::utils::generate_non_fs_fake_path(&folder.uri));
        let found_projects = Project::search_projects(&p, &ctx.options().ignore_paths).await;
        for project in found_projects {
            ctx.add_project(project);
        }
    }
    Ok(())
}
