use std::path::Path;

use lsp_types::{GotoDefinitionParams, Location, LocationLink, ReferenceParams};

use crate::{ServerContext, context::project::Project};

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
                todo!() // TODO
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

mod wxml {
    use glass_easel_template_compiler::parse::{Position, TemplateStructure};

    use crate::wxml_utils::{location_to_lsp_range, ScopeKind, Token};

    use super::*;

    pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position, to_definition: bool) -> anyhow::Result<Vec<LocationLink>> {
        let mut ret = vec![];
        let _ = project.load_wxml_direct_deps(abs_path);
        if let Ok(template) = project.get_wxml_tree(abs_path) {
            let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
            match token {
                Token::ScopeRef(loc, kind) => {
                    match kind {
                        ScopeKind::Script(script) => {
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&loc)),
                                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                target_range: location_to_lsp_range(&script.module_name().location),
                                target_selection_range: location_to_lsp_range(&script.module_name().location),
                            });
                        }
                        ScopeKind::ForScope(target) => {
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&loc)),
                                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                target_range: location_to_lsp_range(&target.location),
                                target_selection_range: location_to_lsp_range(&target.location),
                            });
                        }
                        ScopeKind::SlotValue(elem, attr) => {
                            if to_definition {
                                // TODO
                            } else {
                                ret.push(LocationLink {
                                    origin_selection_range: Some(location_to_lsp_range(&loc)),
                                    target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                    target_range: location_to_lsp_range(&attr.value.location),
                                    target_selection_range: location_to_lsp_range(&attr.value.location),
                                });
                            }
                        }
                    }
                }
                Token::TemplateName(name)
                | Token::ScriptModule(name)
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
                Token::SlotValueScope(name) => {
                    if to_definition {
                        // TODO
                    } else {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&name.location)),
                            target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            target_range: location_to_lsp_range(&name.location),
                            target_selection_range: location_to_lsp_range(&name.location),
                        });
                    }
                }
                Token::SlotValueRef(key) | Token::SlotValueRefAndScope(key) => {
                    // TODO go to target component
                }
                Token::TemplateRef(is, loc) => {
                    if let Some(x) = template.globals.sub_templates.iter().rev().find(|x| x.name.is(is)) {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                            target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            target_range: location_to_lsp_range(&x.name.location()),
                            target_selection_range: location_to_lsp_range(&x.name.location()),
                        });
                    } else {
                        for import in template.globals.imports.iter() {
                            if let Ok(p) = project.find_rel_path_for_file(abs_path, &import.src.name) {
                                let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") else {
                                    continue;
                                };
                                if let Ok(imported_template) = project.get_wxml_tree(&imported_path) {
                                    if let Some(x) = imported_template.globals.sub_templates.iter().rev().find(|x| x.name.is(is)) {
                                        ret.push(LocationLink {
                                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                                            target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                                            target_range: location_to_lsp_range(&x.name.location()),
                                            target_selection_range: location_to_lsp_range(&x.name.location()),
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Token::Src(src) => {
                    if let Ok(p) = project.find_rel_path_for_file(abs_path, &src.name) {
                        if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") {
                            let target_range = lsp_types::Range::new(
                                lsp_types::Position { line: 0, character: 0 },
                                lsp_types::Position { line: 0, character: 0 },
                            );
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&src.location)),
                                target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                                target_range: target_range.clone(),
                                target_selection_range: target_range,
                            });
                        }
                    }
                }
                Token::ScriptSrc(src) => {
                    if let Ok(p) = project.find_rel_path_for_file(abs_path, &src.name) {
                        if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxs") {
                            let target_range = lsp_types::Range::new(
                                lsp_types::Position { line: 0, character: 0 },
                                lsp_types::Position { line: 0, character: 0 },
                            );
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&src.location)),
                                target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                                target_range: target_range.clone(),
                                target_selection_range: target_range,
                            });
                        }
                    }
                }
                Token::ScriptContent(..) => {
                    // TODO pass to wxs ls
                }
                _ => {}
            }
        }
        Ok(ret)
    }
}
