use std::path::{Path, PathBuf};

use lsp_types::{GotoDefinitionParams, Location, LocationLink, ReferenceParams};

use crate::{ServerContext, context::project::Project};

mod wxml;

pub(crate) async fn find_definition(ctx: ServerContext, params: GotoDefinitionParams) -> anyhow::Result<Vec<LocationLink>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position.clone();
    let ret = ctx.clone().project_thread_task(uri, move |project, abs_path| -> anyhow::Result<Vec<LocationLink>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                wxml::find_declaration(project, &abs_path, position, true)?
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

pub(crate) async fn find_declaration(ctx: ServerContext, params: GotoDefinitionParams) -> anyhow::Result<Vec<LocationLink>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position.clone();
    let ret = ctx.clone().project_thread_task(uri, move |project, abs_path| -> anyhow::Result<Vec<LocationLink>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                wxml::find_declaration(project, &abs_path, position, false)?
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

pub(crate) async fn find_references(ctx: ServerContext, params: ReferenceParams) -> anyhow::Result<Vec<Location>> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position.clone();
    let ret = ctx.clone().project_thread_task(&uri, move |project, abs_path| -> anyhow::Result<Vec<Location>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                wxml::find_references(project, &abs_path, position)?
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}
