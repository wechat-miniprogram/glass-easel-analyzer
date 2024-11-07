use std::path::Path;

use lsp_types::{GotoDefinitionParams, Location, LocationLink, ReferenceParams};

use crate::{ServerContext, context::project::Project};

pub(crate) async fn find_definition(ctx: ServerContext, params: GotoDefinitionParams) -> anyhow::Result<Vec<LocationLink>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position.clone();
    let ret = ctx.clone().project_thread_task(uri, move |project, abs_path| -> anyhow::Result<Vec<LocationLink>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                wxml::find_declaration(project, &abs_path, position)?
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
                wxml::find_declaration(project, &abs_path, position)?
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
                todo!()
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

mod wxml {
    use glass_easel_template_compiler::parse::{Position, TemplateStructure};

    use crate::wxml_utils::{location_to_lsp_range, Token};

    use super::*;

    pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<LocationLink>> {
        let mut ret = vec![];
        if let Ok(template) = project.get_wxml_tree(abs_path) {
            let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
            match token {
                Token::ScopeRef(loc, target) => {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&loc)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&target.location),
                        target_selection_range: location_to_lsp_range(&target.location),
                    })
                }
                Token::TemplateName(name)
                | Token::ScriptModule(name)
                | Token::SlotValueScope(name)
                | Token::ForItem(name)
                | Token::ForIndex(name)
                | Token::ForKey(name) => {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&name.location)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&name.location),
                        target_selection_range: location_to_lsp_range(&name.location),
                    });
                }
                Token::SlotValueRefAndScope(name) => {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&name.location)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&name.location),
                        target_selection_range: location_to_lsp_range(&name.location),
                    });
                }
                Token::TemplateRef(is, loc) => {
                    if let Some(x) = template.globals.sub_templates.iter().find(|x| x.name.is(is)) {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                            target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            target_range: location_to_lsp_range(&x.name.location()),
                            target_selection_range: location_to_lsp_range(&x.name.location()),
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(ret)
    }
}
