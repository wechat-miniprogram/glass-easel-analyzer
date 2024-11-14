use std::path::Path;

use glass_easel_template_compiler::parse::{tag::{ElementKind, Ident}, Position};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, CompletionParams, InsertTextFormat};

use crate::{context::{backend_configuration::{AttributeConfig, ComponentConfig, ElementConfig, EventConfig, PropertyConfig}, project::Project}, wxml_utils::{location_to_lsp_range, ScopeKind, Token}, BackendConfig, ServerContext};

pub(crate) async fn completion(ctx: ServerContext, params: CompletionParams) -> anyhow::Result<Option<CompletionList>> {
    let backend_config = ctx.backend_config();
    let ret = ctx.clone().project_thread_task(&params.text_document_position.text_document.uri, move |project, abs_path| -> anyhow::Result<Option<CompletionList>> {
        let list = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                let trigger = params.context.as_ref().and_then(|x| x.trigger_character.as_ref().map(|x| x.as_str())).unwrap_or_default();
                completion_wxml(project, &backend_config, &abs_path, params.text_document_position.position, trigger)
            }
            _ => None,
        };
        Ok(list)
    }).await??;
    Ok(ret)
}

fn completion_wxml(project: &mut Project, backend_config: &BackendConfig, abs_path: &Path, pos: lsp_types::Position, trigger: &str) -> Option<CompletionList> {
    let _ = project.load_wxml_direct_deps(abs_path);
    let template = project.get_wxml_tree(abs_path).ok()?;
    let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
    fn simple_completion_item(s: impl Into<String>, kind: CompletionItemKind) -> CompletionItem {
        CompletionItem {
            label: s.into(),
            kind: Some(kind),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        }
    }
    fn snippet_completion_item(s: impl Into<String>, snippet: impl Into<String>, kind: CompletionItemKind) -> CompletionItem {
        CompletionItem {
            label: s.into(),
            kind: Some(kind),
            insert_text: Some(snippet.into()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }
    match token {
        Token::TagName(tag_name) => {
            let mut items: Vec<CompletionItem> = vec![];
            let before_len = pos.character.saturating_sub(tag_name.location.start.utf16_col) as usize;
            let tag_name_before = &tag_name.name.get(0..before_len).unwrap_or_default();
            if let Some(config) = project.get_json_config(abs_path) {
                for key in config.using_components.keys() {
                    if Ident::is_valid(key) && key.starts_with(tag_name_before) {
                        items.push(simple_completion_item(key, CompletionItemKind::CLASS));
                    }
                }
            }
            for comp in backend_config.component.iter() {
                if comp.tag_name.starts_with(tag_name_before) {
                    items.push(simple_completion_item(&comp.tag_name, CompletionItemKind::CLASS));
                }
            }
            for elem in backend_config.element.iter() {
                if elem.tag_name.starts_with(tag_name_before) {
                    items.push(simple_completion_item(&elem.tag_name, CompletionItemKind::CLASS));
                }
            }
            Some(CompletionList { is_incomplete: false, items })
        }
        Token::StartTagBody(elem) => {
            let mut items: Vec<CompletionItem> = vec![];
            match &elem.kind {
                ElementKind::Normal { tag_name, attributes, .. } => {
                    let has_attr = |name: &str| attributes.iter().find(|x| x.name.name.as_str() == name).is_some();
                    if let Some(_target_path) = project.get_cached_target_component_path(abs_path, &tag_name.name) {
                        // empty
                    } else if let Some(comp) = backend_config.search_component(&tag_name.name) {
                        for prop in comp.property.iter() {
                            let name = &prop.name;
                            if has_attr(name) { continue; }
                            if prop.ty == "boolean" {
                                items.push(simple_completion_item(name, CompletionItemKind::VARIABLE));
                            } else {
                                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
                            }
                        }
                    } else if let Some(elem) = backend_config.search_element(&tag_name.name) {
                        for attr in elem.attribute.iter() {
                            let name = &attr.name;
                            if has_attr(&name) { continue; }
                            items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
                        }
                        for attr in backend_config.global_attribute.iter() {
                            let name = &attr.name;
                            if has_attr(&name) { continue; }
                            items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
                        }
                    }
                    for name in ["class", "style", "id", "slot"] {
                        if has_attr(name) { continue; }
                        items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
                    }
                    // ["generic", "model", "change", "slot", "data", "mark", "bind", "mut-bind", "catch", "capture-bind", "capture-mut-bind", "capture-catch"]
                }
                _ => {}
            }
            Some(CompletionList { is_incomplete: false, items })
        }
        // Token::AttributeName(attr_name, tag_name) => {
        //     // TODO
        // }
        // Token::EventName(event_name, elem) => {
        //     // TODO
        // }
        // Token::ScriptContent(..) => {
        //     // TODO pass to wxs ls
        // }
        _ => None,
    }
}
