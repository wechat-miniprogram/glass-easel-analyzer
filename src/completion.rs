use std::path::Path;

use glass_easel_template_compiler::parse::{tag::{ClassAttribute, CommonElementAttributes, Element, ElementKind, Ident, StyleAttribute}, Position};
use itertools::Itertools;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, CompletionParams, InsertTextFormat};

use crate::{context::project::Project, wxml_utils::Token, BackendConfig, ServerContext};

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

fn completion_wxml(project: &mut Project, backend_config: &BackendConfig, abs_path: &Path, pos: lsp_types::Position, _trigger: &str) -> Option<CompletionList> {
    let _ = project.load_wxml_direct_deps(abs_path);
    let template = project.get_wxml_tree(abs_path).ok()?;
    let file_content = project.cached_file_content(abs_path)?;
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
    let extract_str_before = |loc: &std::ops::Range<Position>, pos: lsp_types::Position| {
        let start_index = file_content.content_index_for_line_utf16_col(loc.start.line, loc.start.utf16_col);
        let index = file_content.content_index_for_line_utf16_col(pos.line, pos.character);
        &file_content.content[start_index..index]
    };
    let handle_attr = |elem: &Element, has_prefix: bool| {
        let mut items: Vec<CompletionItem> = vec![];
        let common = match &elem.kind {
            ElementKind::Normal { common, .. }
            | ElementKind::Slot { common, .. } => {
                Some(common)
            }
            _ => None,
        };
        let has_event = |common: &CommonElementAttributes, name: &str| common.event_bindings.iter().find(|x| x.name.name.as_str() == name).is_some();
        if let ElementKind::Normal { tag_name, attributes, change_attributes, class, style, common, .. } = &elem.kind {
            let has_attr = |name: &str| {
                attributes.iter().chain(change_attributes.iter()).find(|x| x.name.name.as_str() == name).is_some()
            };
            if let Some(_target_path) = project.get_cached_target_component_path(abs_path, &tag_name.name) {
                // empty
            } else if let Some(props) = backend_config.list_properties(&tag_name.name) {
                for prop in props {
                    let name = &prop.name;
                    if has_attr(name) { continue; }
                    if prop.ty == "boolean" {
                        items.push(simple_completion_item(name, CompletionItemKind::VARIABLE));
                    } else {
                        items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::VARIABLE));
                    }
                }
                if !has_prefix {
                    for name in ["model:", "change:"] {
                        let choices = backend_config.list_properties(&tag_name.name).unwrap().map(|x| &x.name).filter(|x| !has_attr(x)).join(",");
                        if choices.len() > 0 {
                            items.push(snippet_completion_item(name, format!("{}${{1|{}|}}=\"$0\"", name, choices), CompletionItemKind::KEYWORD));
                        }
                    }
                }
            } else if let Some(attrs) = backend_config.list_attributes(&tag_name.name) {
                for attr in attrs {
                    let name = &attr.name;
                    if has_attr(&name) { continue; }
                    items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::VARIABLE));
                }
                if !has_prefix {
                    for name in ["model:", "change:"] {
                        let choices = backend_config.list_attributes(&tag_name.name).unwrap().map(|x| &x.name).filter(|x| !has_attr(x)).join(",");
                        if choices.len() > 0 {
                            items.push(snippet_completion_item(name, format!("{}${{1|{}|}}=\"$0\"", name, choices), CompletionItemKind::KEYWORD));
                        }
                    }
                }
            }
            if let ClassAttribute::None = class {
                let name = "class";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            if let StyleAttribute::None = style {
                let name = "style";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            for name in ["generic:"] {
                items.push(snippet_completion_item(name, format!("{}$1=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            for name in ["wx:if", "wx:elif", "wx:else", "wx:for", "wx:for-item", "wx:for-index", "wx:key"] {
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            for name in ["bind:", "mut-bind:", "catch:", "capture-bind:", "capture-mut-bind:", "capture-catch:"] {
                if let Some(choices) = backend_config.list_events(&tag_name.name) {
                    let choices = choices.map(|x| &x.name).filter(|x| !has_event(common, x)).join(",");
                    if choices.len() > 0 {
                        items.push(snippet_completion_item(name, format!("{}${{1|{}|}}=\"$0\"", name, choices), CompletionItemKind::KEYWORD));
                    }
                }
            }
        } else if let ElementKind::Pure { .. } = &elem.kind {
            for name in ["wx:if", "wx:elif", "wx:else", "wx:for"] {
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
        } else if let ElementKind::For { list, item_name, index_name, key, .. } = &elem.kind {
            if item_name.0.start == list.0.start {
                let name = "wx:for-item";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            if index_name.0.start == list.0.start {
                let name = "wx:for-index";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            if key.0.start == list.0.start {
                let name = "wx:key";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
        } else {
            if let Some(common) = common {
                for name in ["bind:", "mut-bind:", "catch:", "capture-bind:", "capture-mut-bind:", "capture-catch:"] {
                    let choices = backend_config.list_global_events();
                    let choices = choices.map(|x| &x.name).filter(|x| !has_event(common, x)).join(",");
                    if choices.len() > 0 {
                        items.push(snippet_completion_item(name, format!("{}${{1|{}|}}=\"$0\"", name, choices), CompletionItemKind::KEYWORD));
                    }
                }
            }
        }
        if let Some(common) = common {
            if common.id.is_none() {
                let name = "id";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            if common.slot.is_none() {
                let name = "slot";
                items.push(snippet_completion_item(name, format!("{}=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
            for name in ["slot:", "data:", "mark:"] {
                items.push(snippet_completion_item(name, format!("{}$1=\"$0\"", name), CompletionItemKind::KEYWORD));
            }
        }
        Some(CompletionList { is_incomplete: false, items })
    };
    match token {
        Token::StaticTextContent(loc, _s, _parent) => {
            let s_before = extract_str_before(&loc, pos);
            if s_before.ends_with("<") {
                let mut items: Vec<CompletionItem> = vec![];
                if let Some(config) = project.get_json_config(abs_path) {
                    for key in config.using_components.keys() {
                        if Ident::is_valid(key) {
                            items.push(snippet_completion_item(key, format!("{key}>$0</{key}>", key = key), CompletionItemKind::CLASS));
                        }
                    }
                }
                for comp in backend_config.component.iter() {
                    let name = &comp.tag_name;
                    items.push(snippet_completion_item(name, format!("{key}>$0</{key}>", key = name), CompletionItemKind::CLASS));
                }
                for elem in backend_config.element.iter() {
                    let name = &elem.tag_name;
                    items.push(snippet_completion_item(name, format!("{key}>$0</{key}>", key = name), CompletionItemKind::CLASS));
                }
                for key in ["slot", "block"] {
                    items.push(snippet_completion_item(key, format!("{key}>$0</{key}>", key = key), CompletionItemKind::KEYWORD));
                }
                for key in ["include", "import"] {
                    items.push(snippet_completion_item(key, format!("{key} src=\"$0\" />", key = key), CompletionItemKind::KEYWORD));
                }
                items.push(snippet_completion_item("template name", format!("template name=\"$1\">$0</template>"), CompletionItemKind::KEYWORD));
                if let Some(choices) = project.get_cached_wxml_template_names(abs_path) {
                    if choices.len() > 0 {
                        let choices = choices.join(",");
                        items.push(snippet_completion_item("template is", format!("template is=\"${{1|{}|}}\" data=\"{{{{ $0 }}}}\" />", choices), CompletionItemKind::KEYWORD));
                    }
                }
                Some(CompletionList { is_incomplete: false, items })
            } else {
                None
            }
        }
        Token::EndTagBody(elem) => {
            match &elem.kind {
                ElementKind::Normal { tag_name, .. } => {
                    let mut items: Vec<CompletionItem> = vec![];
                    items.push(simple_completion_item(format!("{}>", tag_name.name), CompletionItemKind::CLASS));
                    Some(CompletionList { is_incomplete: false, items })
                }
                _ => None
            }
        }
        Token::TagName(_tag_name) => {
            let mut items: Vec<CompletionItem> = vec![];
            if let Some(config) = project.get_json_config(abs_path) {
                for key in config.using_components.keys() {
                    if Ident::is_valid(key) {
                        items.push(simple_completion_item(key, CompletionItemKind::CLASS));
                    }
                }
            }
            for comp in backend_config.component.iter() {
                items.push(simple_completion_item(&comp.tag_name, CompletionItemKind::CLASS));
            }
            for elem in backend_config.element.iter() {
                items.push(simple_completion_item(&elem.tag_name, CompletionItemKind::CLASS));
            }
            Some(CompletionList { is_incomplete: false, items })
        }
        Token::StartTagBody(elem) => {
            handle_attr(elem, false)
        }
        Token::AttributeName(_attr_name, elem) => {
            handle_attr(elem, false)
        }
        Token::ModelAttributeName(_attr_name, elem) => {
            handle_attr(elem, true)
        }
        Token::ChangeAttributeName(_attr_name, elem) => {
            handle_attr(elem, true)
        }
        Token::AttributeKeyword(_loc, elem) => {
            handle_attr(elem, false)
        }
        Token::EventName(_event_name, elem) => {
            let mut items: Vec<CompletionItem> = vec![];
            let has_event = |common: &CommonElementAttributes, name: &str| common.event_bindings.iter().find(|x| x.name.name.as_str() == name).is_some();
            let common = match &elem.kind {
                ElementKind::Normal { common, .. }
                | ElementKind::Slot { common, .. } => {
                    Some(common)
                }
                _ => None,
            };
            if let Some(common) = common {    
                let tag_name = match &elem.kind {
                    ElementKind::Normal { tag_name, .. } => Some(tag_name),
                    _ => None
                };
                if let Some(events) = tag_name.and_then(|x| backend_config.list_events(&x.name)) {
                    for ev in events {
                        if has_event(common, &ev.name) { continue; }
                        items.push(simple_completion_item(&ev.name, CompletionItemKind::VARIABLE));
                    }
                } else {
                    let events = backend_config.list_global_events();
                    for ev in events {
                        if has_event(common, &ev.name) { continue; }
                        items.push(simple_completion_item(&ev.name, CompletionItemKind::VARIABLE));
                    }
                };
            }
            Some(CompletionList { is_incomplete: false, items })
        }
        Token::TemplateRef(_name, _loc) => {
            let mut items: Vec<CompletionItem> = vec![];
            if let Some(choices) = project.get_cached_wxml_template_names(abs_path) {
                if choices.len() > 0 {
                    let choices = choices.join(",");
                    items.push(snippet_completion_item("template is", format!("template is=\"${{1|{}|}}\" data=\"{{{{ $0 }}}}\" />", choices), CompletionItemKind::KEYWORD));
                }
            }
            Some(CompletionList { is_incomplete: false, items })
        }
        // Token::ScriptContent(..) => {
        //     // TODO pass to wxs ls
        // }
        _ => None,
    }
}
