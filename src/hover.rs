use std::path::Path;

use glass_easel_template_compiler::parse::{tag::ElementKind, Position};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

use crate::{context::{backend_configuration::{AttributeConfig, ComponentConfig, ElementConfig, EventConfig, PropertyConfig}, project::Project}, wxml_utils::{location_to_lsp_range, ScopeKind, Token}, BackendConfig, ServerContext};

pub(crate) async fn hover(ctx: ServerContext, params: HoverParams) -> anyhow::Result<Option<Hover>> {
    let backend_config = ctx.backend_config();
    let ret = ctx.clone().project_thread_task(&params.text_document_position_params.text_document.uri, move |project, abs_path| -> anyhow::Result<Option<Hover>> {
        let hover = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                hover_wxml(project, &backend_config, &abs_path, params.text_document_position_params.position)
            }
            _ => None,
        };
        Ok(hover)
    }).await??;
    Ok(ret)
}

fn hover_wxml(project: &mut Project, backend_config: &BackendConfig, abs_path: &Path, pos: lsp_types::Position) -> Option<Hover> {
    let template = project.get_wxml_tree(abs_path).ok()?;
    let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
    fn plain_str_hover_contents(s: impl Into<String>) -> HoverContents {
        HoverContents::Markup(MarkupContent { kind: MarkupKind::PlainText, value: s.into() })
    }
    fn md_str_hover_contents(s: impl Into<String>) -> HoverContents {
        HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value: s.into() })
    }
    match token {
        Token::ScopeRef(loc, kind) => {
            let contents = match kind {
                ScopeKind::Script(_) => plain_str_hover_contents("wxs script"),
                ScopeKind::ForScope(_, _) => plain_str_hover_contents("wx:for scope"),
                ScopeKind::SlotValue(_, _) => plain_str_hover_contents("slot value"),
            };
            Some(Hover { contents, range: Some(location_to_lsp_range(&loc)) })
        }
        Token::TagName(tag_name) => {
            let contents = if let Some(_target_path) = project.get_target_component_path(abs_path, &tag_name.name) {
                plain_str_hover_contents("custom component")
            } else if let Some(elem) = backend_config.search_component(&tag_name.name) {
                let ComponentConfig { tag_name, description, reference, .. } = elem;
                let reference_args = if let Some(r) = reference {
                    format!("\n\n[Reference]({})", r)
                } else {
                    format!("")
                };
                md_str_hover_contents(format!("**{}** *component*\n\n{}{}", tag_name, description, reference_args))
            } else if let Some(elem) = backend_config.search_element(&tag_name.name) {
                let ElementConfig { tag_name, description, reference, .. } = elem;
                let reference_args = if let Some(r) = reference {
                    format!("\n\n[Reference]({})", r)
                } else {
                    format!("")
                };
                md_str_hover_contents(format!("**{}** *element*\n\n{}{}", tag_name, description, reference_args))
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover { contents, range: Some(location_to_lsp_range(&tag_name.location)) })
        }
        Token::AttributeName(attr_name, elem) => {
            let tag_name = match &elem.kind {
                ElementKind::Normal { tag_name, .. } => Some(tag_name),
                _ => None
            };
            let contents = if let Some(tag_name) = tag_name {
                if let Some(_target_path) = project.get_target_component_path(abs_path, &tag_name.name) {
                    plain_str_hover_contents("custom component property")
                } else if let Some(prop) = backend_config.search_property(&tag_name.name, &attr_name.name) {
                    let PropertyConfig { name, ty, description, reference, .. } = prop;
                    let ty_args = if ty.len() > 0 {
                        format!(": {}", ty)
                    } else {
                        format!("")
                    };
                    let reference_args = if let Some(r) = reference {
                        format!("\n\n[Reference]({})", r)
                    } else {
                        format!("")
                    };
                    md_str_hover_contents(format!("**{}**{} *property*\n\n{}{}", name, ty_args, description, reference_args))
                } else if let Some(attr) = backend_config.search_attribute(&tag_name.name, &attr_name.name) {
                    let AttributeConfig { name, description, reference, .. } = attr;
                    let reference_args = if let Some(r) = reference {
                        format!("\n\n[Reference]({})", r)
                    } else {
                        format!("")
                    };
                    md_str_hover_contents(format!("**{}** *attribute*\n\n{}{}", name, description, reference_args))
                } else {
                    plain_str_hover_contents("unknown")
                }
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover { contents, range: Some(location_to_lsp_range(&attr_name.location)) })
        }
        Token::EventName(event_name, elem) => {
            let tag_name = match &elem.kind {
                ElementKind::Normal { tag_name, .. } => Some(tag_name),
                _ => None
            };
            let contents = if let Some(tag_name) = tag_name {
                if let Some(_target_path) = project.get_target_component_path(abs_path, &tag_name.name) {
                    plain_str_hover_contents("custom component property")
                } else if let Some(ev) = backend_config.search_event(&tag_name.name, &event_name.name) {
                    let EventConfig { name, description, reference, .. } = ev;
                    let reference_args = if let Some(r) = reference {
                        format!("\n\n[Reference]({})", r)
                    } else {
                        format!("")
                    };
                    md_str_hover_contents(format!("**{}** *event*\n\n{}{}", name, description, reference_args))
                } else {
                    plain_str_hover_contents("unknown")
                }
            } else if let Some(ev) = backend_config.search_global_event(&event_name.name) {
                let EventConfig { name, description, reference, .. } = ev;
                let reference_args = if let Some(r) = reference {
                    format!("\n\n[Reference]({})", r)
                } else {
                    format!("")
                };
                md_str_hover_contents(format!("**{}** *event*\n\n{}{}", name, description, reference_args))
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover { contents, range: Some(location_to_lsp_range(&event_name.location)) })
        }
        // Token::ScriptContent(..) => {
        //     // TODO pass to wxs ls
        // }
        _ => None,
    }
}
