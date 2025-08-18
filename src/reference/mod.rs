use std::path::{Path, PathBuf};

use lsp_types::{GotoDefinitionParams, Location, LocationLink, ReferenceParams};

use crate::{
    context::{project::Project, FileLang},
    ServerContext,
};

mod wxml;
mod wxss;

pub(crate) async fn find_definition(
    ctx: ServerContext,
    params: GotoDefinitionParams,
) -> anyhow::Result<Vec<LocationLink>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position.clone();
    let ret = ctx
        .clone()
        .project_thread_task(
            uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<LocationLink>> {
                let ranges = match file_lang {
                    FileLang::Wxml => wxml::find_declaration(project, &abs_path, position, true)?,
                    FileLang::Wxss | FileLang::OtherSs => wxss::find_declaration(project, &abs_path, position)?,
                    _ => vec![],
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}

pub(crate) async fn find_declaration(
    ctx: ServerContext,
    params: GotoDefinitionParams,
) -> anyhow::Result<Vec<LocationLink>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position.clone();
    let ret = ctx
        .clone()
        .project_thread_task(
            uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<LocationLink>> {
                let ranges = match file_lang {
                    FileLang::Wxml => wxml::find_declaration(project, &abs_path, position, false)?,
                    FileLang::Wxss | FileLang::OtherSs => wxss::find_declaration(project, &abs_path, position)?,
                    _ => vec![],
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}

pub(crate) async fn find_references(
    ctx: ServerContext,
    params: ReferenceParams,
) -> anyhow::Result<Vec<Location>> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position.clone();
    let ret = ctx
        .clone()
        .project_thread_task(
            &uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<Location>> {
                let ranges = match file_lang {
                    FileLang::Wxml => wxml::find_references(project, &abs_path, position)?,
                    FileLang::Wxss | FileLang::OtherSs => wxss::find_references(project, &abs_path, position)?,
                    _ => vec![],
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}
