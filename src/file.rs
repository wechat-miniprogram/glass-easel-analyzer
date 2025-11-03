use std::path::Path;

use glass_easel_template_compiler::parse::tag::Script;
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileChangeType, PublishDiagnosticsParams, TextDocumentContentChangeEvent, Url,
};
use serde::Deserialize;

use crate::{
    context::{project::Project, FileLang},
    utils::log_if_err,
    ServerContext,
};

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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineWxsScripts<'a> {
    uri: &'a str,
    list: Vec<InlineWxsScript<'a>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineWxsScript<'a> {
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
    content: &'a str,
}

fn update_inline_wxs_list(ctx: &ServerContext, project: &Project, uri: &Url, abs_path: &Path) {
    if let Ok(x) = project.get_wxml_tree(&abs_path) {
        let scripts = x
            .globals
            .scripts
            .iter()
            .filter_map(|x| match x {
                Script::Inline {
                    content,
                    content_location,
                    ..
                } => {
                    let script = InlineWxsScript {
                        start_line: content_location.start.line as u32,
                        start_column: content_location.start.utf16_col as u32,
                        end_line: content_location.end.line as u32,
                        end_column: content_location.end.utf16_col as u32,
                        content,
                    };
                    Some(script)
                }
                _ => None,
            })
            .collect();
        log_if_err(ctx.send_notification(
            "glassEaselAnalyzer/inlineWxsScripts",
            InlineWxsScripts {
                uri: uri.as_str(),
                list: scripts,
            },
        ));
    }
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
                    "wxml" => {
                        let diag = project.open_wxml(&abs_path, params.text_document.text);
                        update_inline_wxs_list(&ctx, project, &uri, &abs_path);
                        diag
                    }
                    "wxss" => project.open_wxss(&abs_path, params.text_document.text),
                    "json" => project.open_json(&abs_path, params.text_document.text),
                    "css" | "less" | "scss" => {
                        project.open_other_ss(&abs_path, params.text_document.text)
                    }
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
            .project_thread_task(
                &params.text_document.uri,
                move |project, abs_path, file_lang| {
                    if let Some(content) = project.cached_file_content(&abs_path) {
                        let new_content = apply_content_changes_to_content(
                            &content.content,
                            params.content_changes,
                        );
                        let diag = match file_lang {
                            FileLang::Wxml => {
                                let diag = project.open_wxml(&abs_path, new_content);
                                update_inline_wxs_list(&ctx, project, &uri, &abs_path);
                                diag
                            }
                            FileLang::Wxss => project.open_wxss(&abs_path, new_content),
                            FileLang::Json => project.open_json(&abs_path, new_content),
                            FileLang::OtherSs => project.open_other_ss(&abs_path, new_content),
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
                },
            )
            .await,
    );
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestDiagnosticsParams {
    text_document_uri: Url,
}

pub(crate) async fn request_diagnostics(
    ctx: ServerContext,
    params: RequestDiagnosticsParams,
) -> anyhow::Result<bool> {
    let uri = params.text_document_uri.clone();
    ctx
        .clone()
        .project_thread_task(
            &params.text_document_uri,
            move |project, abs_path, file_lang| {
                if let Some(content) = project.cached_file_content_if_opened(&abs_path) {
                    let content = content.content.clone();
                    let diag = match file_lang {
                        FileLang::Wxml => project.open_wxml(&abs_path, content),
                        FileLang::Wxss => project.open_wxss(&abs_path, content),
                        FileLang::Json => project.open_json(&abs_path, content),
                        FileLang::OtherSs => project.open_other_ss(&abs_path, content),
                        _ => return false,
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
                    true
                } else {
                    false
                }
            },
        )
        .await
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
        let found_projects = Project::search_projects(&p, ctx.options()).await;
        for mut project in found_projects {
            project.init().await;
            if let Some(path) = project.root().and_then(|x| x.to_str()) {
                ctx.send_notification(
                    "glassEaselAnalyzer/discoveredProject",
                    crate::ProjectInfo { path },
                )
                .unwrap();
            }
            ctx.add_project(project);
        }
    }
    Ok(())
}
