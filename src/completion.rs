use std::{collections::HashSet, path::Path};

use glass_easel_template_compiler::parse::{
    tag::{
        ClassAttribute, CommonElementAttributes, Element, ElementKind, Ident, StyleAttribute, Value,
    },
    Position,
};
use itertools::Itertools;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionParams, InsertTextFormat,
};

use crate::{
    context::{
        backend_configuration::MediaFeatureType,
        project::{FileContentMetadata, Project},
        FileLang,
    },
    wxml_utils::{
        for_each_static_class_name_in_element, for_each_template_element, Token as WxmlToken, TokenStaticStyleValuePart,
    },
    wxss::rule::Selector,
    wxss_utils::{for_each_selector_in_style_sheet, Token as WxssToken},
    BackendConfig, ServerContext,
};

fn collect_ids_in_wxml(project: &Project, abs_path: &Path) -> HashSet<String> {
    let mut item_set = HashSet::new();
    let wxml_path = abs_path.with_extension("wxml");
    if let Ok(template) = project.get_wxml_tree(&wxml_path) {
        project.import_and_include_templates(abs_path, template, |_, template| {
            for_each_template_element(template, |elem, _| {
                if let ElementKind::Normal { common, .. } = &elem.kind {
                    if let Some((_, Value::Static { value, .. })) = common.id.as_ref() {
                        item_set.insert(value.to_string());
                    }
                }
            });
        });
    }
    item_set
}

fn collect_classes_in_wxml(project: &Project, abs_path: &Path) -> HashSet<String> {
    let mut item_set = HashSet::new();
    let wxml_path = abs_path.with_extension("wxml");
    if let Ok(template) = project.get_wxml_tree(&wxml_path) {
        project.import_and_include_templates(abs_path, template, |_, template| {
            for_each_template_element(template, |elem, _| {
                for_each_static_class_name_in_element(elem, |class_name, _| {
                    item_set.insert(class_name.to_string());
                });
            });
        });
    }
    item_set
}

fn collect_ids_in_wxss(project: &Project, abs_path: &Path) -> HashSet<String> {
    let mut item_set = HashSet::new();
    let wxss_path = abs_path.with_extension("wxss");
    if let Ok(sheet) = project.get_style_sheet(&wxss_path) {
        project.import_style_sheets(abs_path, sheet, |_, sheet| {
            for_each_selector_in_style_sheet(sheet, |sel| {
                if let Selector::Id(x) = sel {
                    item_set.insert(x.content.to_string());
                }
            });
        });
    }
    item_set
}

fn collect_classes_in_wxss(project: &Project, abs_path: &Path) -> HashSet<String> {
    let mut item_set = HashSet::new();
    let wxss_path = abs_path.with_extension("wxss");
    if let Ok(sheet) = project.get_style_sheet(&wxss_path) {
        project.import_style_sheets(abs_path, sheet, |_, sheet| {
            for_each_selector_in_style_sheet(sheet, |sel| {
                if let Selector::Class(_, x) = sel {
                    item_set.insert(x.content.to_string());
                }
            });
        });
    }
    item_set
}

pub(crate) async fn completion(
    ctx: ServerContext,
    params: CompletionParams,
) -> anyhow::Result<Option<CompletionList>> {
    let backend_config = ctx.backend_config();
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_position.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Option<CompletionList>> {
                let list = match file_lang {
                    FileLang::Wxml => {
                        let trigger = params
                            .context
                            .as_ref()
                            .and_then(|x| x.trigger_character.as_ref().map(|x| x.as_str()))
                            .unwrap_or_default();
                        completion_wxml(
                            project,
                            &backend_config,
                            &abs_path,
                            params.text_document_position.position,
                            trigger,
                        )
                    }
                    FileLang::Wxss => {
                        let trigger = params
                            .context
                            .as_ref()
                            .and_then(|x| x.trigger_character.as_ref().map(|x| x.as_str()))
                            .unwrap_or_default();
                        completion_wxss(
                            project,
                            &backend_config,
                            &abs_path,
                            params.text_document_position.position,
                            trigger,
                        )
                    }
                    _ => None,
                };
                Ok(list)
            },
        )
        .await??;
    Ok(ret)
}

fn simple_completion_item(
    s: impl Into<String>,
    kind: CompletionItemKind,
    deprecated: bool,
) -> CompletionItem {
    CompletionItem {
        label: s.into(),
        kind: Some(kind),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        deprecated: if deprecated { Some(deprecated) } else { None },
        ..Default::default()
    }
}

fn snippet_completion_item(
    s: impl Into<String>,
    snippet: impl Into<String>,
    kind: CompletionItemKind,
    deprecated: bool,
) -> CompletionItem {
    CompletionItem {
        label: s.into(),
        kind: Some(kind),
        insert_text: Some(snippet.into()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        deprecated: if deprecated { Some(deprecated) } else { None },
        ..Default::default()
    }
}

fn extract_str_before(
    file_content: &FileContentMetadata,
    loc: std::ops::Range<Position>,
    pos: lsp_types::Position,
) -> &str {
    let start_index =
        file_content.content_index_for_line_utf16_col(loc.start.line, loc.start.utf16_col);
    let index = file_content.content_index_for_line_utf16_col(pos.line, pos.character);
    &file_content.content[start_index..index]
}

fn completion_wxml(
    project: &mut Project,
    backend_config: &BackendConfig,
    abs_path: &Path,
    pos: lsp_types::Position,
    _trigger: &str,
) -> Option<CompletionList> {
    let template = project.get_wxml_tree(abs_path).ok()?;
    let file_content = project.cached_file_content(abs_path)?;
    let token = crate::wxml_utils::find_token_in_position(
        template,
        Position {
            line: pos.line,
            utf16_col: pos.character,
        },
    );
    let handle_attr = |elem: &Element, has_prefix: bool| {
        let mut items: Vec<CompletionItem> = vec![];
        let common = match &elem.kind {
            ElementKind::Normal { common, .. } | ElementKind::Slot { common, .. } => Some(common),
            _ => None,
        };
        let has_event = |common: &CommonElementAttributes, name: &str| {
            common
                .event_bindings
                .iter()
                .find(|x| x.name.name.as_str() == name)
                .is_some()
        };
        if let ElementKind::Normal {
            tag_name,
            attributes,
            change_attributes,
            class,
            style,
            common,
            ..
        } = &elem.kind
        {
            let has_attr = |name: &str| {
                attributes
                    .iter()
                    .map(|x| &x.name)
                    .chain(change_attributes.iter().map(|x| &x.name))
                    .find(|x| x.name.as_str() == name)
                    .is_some()
            };
            if let Some(_target_path) = project.get_target_component_path(abs_path, &tag_name.name)
            {
                // empty
            } else if let Some(props) = backend_config.list_properties(&tag_name.name) {
                for prop in props {
                    let name = &prop.name;
                    if has_attr(name) {
                        continue;
                    }
                    if prop.ty == "boolean" {
                        items.push(simple_completion_item(
                            name,
                            CompletionItemKind::VARIABLE,
                            prop.deprecated,
                        ));
                    } else {
                        items.push(snippet_completion_item(
                            name,
                            format!("{}=\"$0\"", name),
                            CompletionItemKind::VARIABLE,
                            prop.deprecated,
                        ));
                    }
                }
                if !has_prefix {
                    for name in ["model:", "change:"] {
                        let choices = backend_config
                            .list_properties(&tag_name.name)
                            .unwrap()
                            .map(|x| &x.name)
                            .filter(|x| !has_attr(x))
                            .join(",");
                        if choices.len() > 0 {
                            items.push(snippet_completion_item(
                                name,
                                format!("{}${{1|{}|}}=\"{{{{ $0 }}}}\"", name, choices),
                                CompletionItemKind::KEYWORD,
                                false,
                            ));
                        }
                    }
                }
            } else if let Some(attrs) = backend_config.list_attributes(&tag_name.name) {
                for attr in attrs {
                    let name = &attr.name;
                    if has_attr(&name) {
                        continue;
                    }
                    items.push(snippet_completion_item(
                        name,
                        format!("{}=\"$0\"", name),
                        CompletionItemKind::VARIABLE,
                        attr.deprecated,
                    ));
                }
                if !has_prefix {
                    for name in ["model:", "change:"] {
                        let choices = backend_config
                            .list_attributes(&tag_name.name)
                            .unwrap()
                            .map(|x| &x.name)
                            .filter(|x| !has_attr(x))
                            .join(",");
                        if choices.len() > 0 {
                            items.push(snippet_completion_item(
                                name,
                                format!("{}${{1|{}|}}=\"{{{{ $0 }}}}\"", name, choices),
                                CompletionItemKind::KEYWORD,
                                false,
                            ));
                        }
                    }
                }
            }
            {
                let name = "class";
                let item_set = collect_classes_in_wxss(project, abs_path);
                if item_set.is_empty() {
                    if let ClassAttribute::None = class {
                        items.push(snippet_completion_item(
                            name,
                            format!("{}=\"$0\"", name),
                            CompletionItemKind::KEYWORD,
                            false,
                        ));
                    }
                    items.push(snippet_completion_item(
                        "class:",
                        format!("{}:$0", name),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                } else {
                    let choices = item_set
                        .into_iter()
                        .filter(|x| match class {
                            ClassAttribute::Multiple(arr) => {
                                arr.iter().find(|y| y.1.name == x).is_none()
                            }
                            _ => true,
                        })
                        .join(",");
                    if let ClassAttribute::None = class {
                        items.push(snippet_completion_item(
                            name,
                            format!("{}=\"${{1|{}|}}\"$0", name, choices),
                            CompletionItemKind::KEYWORD,
                            false,
                        ));
                    }
                    items.push(snippet_completion_item(
                        "class:",
                        format!("{}:${{1|{}|}}$0", name, choices),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
            }
            {
                let name = "style";
                if let StyleAttribute::None = style {
                    items.push(snippet_completion_item(
                        name,
                        format!("{}=\"$0\"", name),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
                let choices = backend_config
                    .style_property
                    .iter()
                    .map(|x| &x.name)
                    .filter(|x| match style {
                        StyleAttribute::Multiple(arr) => {
                            arr.iter().find(|y| y.1.name == x).is_none()
                        }
                        _ => true,
                    })
                    .join(",");
                items.push(snippet_completion_item(
                    "style:",
                    format!("{}:${{1|{}|}}=\"$0\"", name, choices),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            for name in ["generic:"] {
                items.push(snippet_completion_item(
                    name,
                    format!("{}$1=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            for name in [
                "wx:if",
                "wx:elif",
                "wx:else",
                "wx:for",
                "wx:for-item",
                "wx:for-index",
                "wx:key",
            ] {
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            for name in [
                "bind:",
                "mut-bind:",
                "catch:",
                "capture-bind:",
                "capture-mut-bind:",
                "capture-catch:",
            ] {
                let choices = if let Some(choices) = backend_config.list_events(&tag_name.name) {
                    choices
                        .map(|x| &x.name)
                        .filter(|x| !has_event(common, x))
                        .join(",")
                } else {
                    let choices = backend_config.list_global_events();
                    choices
                        .map(|x| &x.name)
                        .filter(|x| !has_event(common, x))
                        .join(",")
                };
                if choices.len() > 0 {
                    items.push(snippet_completion_item(
                        name,
                        format!("{}${{1|{}|}}=\"$0\"", name, choices),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
            }
            items.push(snippet_completion_item(
                "let:",
                "let:$1=\"{{ $0 }}\"",
                CompletionItemKind::KEYWORD,
                false,
            ));
        } else if let ElementKind::Pure { .. } = &elem.kind {
            for name in ["wx:if", "wx:elif", "wx:else", "wx:for"] {
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            items.push(snippet_completion_item(
                "let:",
                "let:$1=\"{{ $0 }}\"",
                CompletionItemKind::KEYWORD,
                false,
            ));
        } else if let ElementKind::For {
            list,
            item_name,
            index_name,
            key,
            ..
        } = &elem.kind
        {
            if item_name.0.start == list.0.start {
                let name = "wx:for-item";
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            if index_name.0.start == list.0.start {
                let name = "wx:for-index";
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            if key.0.start == list.0.start {
                let name = "wx:key";
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
        } else {
            if let Some(common) = common {
                for name in [
                    "bind:",
                    "mut-bind:",
                    "catch:",
                    "capture-bind:",
                    "capture-mut-bind:",
                    "capture-catch:",
                ] {
                    let choices = backend_config.list_global_events();
                    let choices = choices
                        .map(|x| &x.name)
                        .filter(|x| !has_event(common, x))
                        .join(",");
                    if choices.len() > 0 {
                        items.push(snippet_completion_item(
                            name,
                            format!("{}${{1|{}|}}=\"$0\"", name, choices),
                            CompletionItemKind::KEYWORD,
                            false,
                        ));
                    }
                }
            }
        }
        if let Some(common) = common {
            if common.id.is_none() {
                let name = "id";
                let item_set = collect_ids_in_wxss(project, abs_path);
                if item_set.is_empty() {
                    items.push(snippet_completion_item(
                        name,
                        format!("{}=\"$0\"", name),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                } else {
                    let choices = item_set.into_iter().join(",");
                    items.push(snippet_completion_item(
                        name,
                        format!("{}=\"${{1|{}|}}\"$0", name, choices),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
            }
            if common.slot.is_none() {
                let name = "slot";
                items.push(snippet_completion_item(
                    name,
                    format!("{}=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
            for name in ["slot:", "data:", "mark:"] {
                items.push(snippet_completion_item(
                    name,
                    format!("{}$1=\"$0\"", name),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
            }
        }
        Some(CompletionList {
            is_incomplete: false,
            items,
        })
    };
    match token {
        WxmlToken::StaticTextContent(loc, _s, _parent) => {
            let s_before = extract_str_before(file_content, loc, pos);
            if s_before.ends_with("<") {
                let mut items: Vec<CompletionItem> = vec![];
                if let Some(config) = project.get_json_config(abs_path) {
                    for key in config.using_components.keys() {
                        if Ident::is_valid(key) {
                            items.push(snippet_completion_item(
                                key,
                                format!("{key}>$0</{key}>", key = key),
                                CompletionItemKind::CLASS,
                                false,
                            ));
                        }
                    }
                }
                for comp in backend_config.component.iter() {
                    let name = &comp.tag_name;
                    items.push(snippet_completion_item(
                        name,
                        format!("{key}>$0</{key}>", key = name),
                        CompletionItemKind::CLASS,
                        comp.deprecated,
                    ));
                }
                for elem in backend_config.element.iter() {
                    let name = &elem.tag_name;
                    items.push(snippet_completion_item(
                        name,
                        format!("{key}>$0</{key}>", key = name),
                        CompletionItemKind::CLASS,
                        elem.deprecated,
                    ));
                }
                for key in ["slot", "block"] {
                    items.push(snippet_completion_item(
                        key,
                        format!("{key}>$0</{key}>", key = key),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
                for key in ["include", "import"] {
                    items.push(snippet_completion_item(
                        key,
                        format!("{key} src=\"$0\" />", key = key),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
                items.push(snippet_completion_item(
                    "template name",
                    format!("template name=\"$1\">$0</template>"),
                    CompletionItemKind::KEYWORD,
                    false,
                ));
                if let Some(choices) = project.get_wxml_template_names(abs_path) {
                    if choices.len() > 0 {
                        let choices = choices.join(",");
                        items.push(snippet_completion_item(
                            "template is",
                            format!(
                                "template is=\"${{1|{}|}}\" data=\"{{{{ $0 }}}}\" />",
                                choices
                            ),
                            CompletionItemKind::KEYWORD,
                            false,
                        ));
                    }
                }
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            } else {
                None
            }
        }
        WxmlToken::EndTagBody(elem) => match &elem.kind {
            ElementKind::Normal { tag_name, .. } => {
                let mut items: Vec<CompletionItem> = vec![];
                items.push(simple_completion_item(
                    format!("{}>", tag_name.name),
                    CompletionItemKind::CLASS,
                    false,
                ));
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }
            _ => None,
        },
        WxmlToken::TagName(_tag_name) => {
            let mut items: Vec<CompletionItem> = vec![];
            if let Some(config) = project.get_json_config(abs_path) {
                for key in config.using_components.keys() {
                    if Ident::is_valid(key) {
                        items.push(simple_completion_item(
                            key,
                            CompletionItemKind::CLASS,
                            false,
                        ));
                    }
                }
            }
            for comp in backend_config.component.iter() {
                items.push(simple_completion_item(
                    &comp.tag_name,
                    CompletionItemKind::CLASS,
                    comp.deprecated,
                ));
            }
            for elem in backend_config.element.iter() {
                items.push(simple_completion_item(
                    &elem.tag_name,
                    CompletionItemKind::CLASS,
                    elem.deprecated,
                ));
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        WxmlToken::StartTagBody(elem) => handle_attr(elem, false),
        WxmlToken::AttributeName(_attr_name, elem) => handle_attr(elem, false),
        WxmlToken::ModelAttributeName(_attr_name, elem) => handle_attr(elem, true),
        WxmlToken::ChangeAttributeName(_attr_name, elem) => handle_attr(elem, true),
        WxmlToken::AttributeKeyword(_loc, elem) => handle_attr(elem, false),
        WxmlToken::StaticClassName(_loc, _name, elem) => match &elem.kind {
            ElementKind::Normal { class, .. } => {
                let items = collect_classes_in_wxss(project, abs_path)
                    .into_iter()
                    .filter(|x| match class {
                        ClassAttribute::Multiple(arr) => {
                            arr.iter().find(|y| y.1.name == x).is_none()
                        }
                        _ => true,
                    })
                    .map(|x| simple_completion_item(x, CompletionItemKind::PROPERTY, false))
                    .collect();
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }
            _ => None,
        },
        WxmlToken::StaticStylePropertyName(_, elem) => match &elem.kind {
            ElementKind::Normal { style, .. } => {
                let items = backend_config
                    .style_property
                    .iter()
                    .map(|x| &x.name)
                    .filter(|x| match style {
                        StyleAttribute::Multiple(arr) => {
                            arr.iter().find(|y| y.1.name == x).is_none()
                        }
                        _ => true,
                    })
                    .map(|x| simple_completion_item(x, CompletionItemKind::PROPERTY, false))
                    .collect();
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }
            _ => None,
        },
        WxmlToken::StaticStylePropertyValue(_, _, name, _) => {
            if let Some(config) = backend_config.style_property.iter().find(|x| x.name == name.name) {
                let mut items = vec![];
                for option in config.options.iter() {
                    items.push(simple_completion_item(
                        option,
                        CompletionItemKind::VALUE,
                        false,
                    ));
                }
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            } else {
                None
            }
        }
        WxmlToken::StaticStyleValuePart(part, elem) => {
            match part {
                TokenStaticStyleValuePart::PropertyName(_, _)
                | TokenStaticStyleValuePart::UnknownIdent(_, _) => match &elem.kind {
                    ElementKind::Normal { style, .. } => {
                        let items = backend_config
                            .style_property
                            .iter()
                            .filter(|x| match style {
                                StyleAttribute::Multiple(arr) => {
                                    arr.iter().find(|y| y.1.name == x.name).is_none()
                                }
                                _ => true,
                            })
                            .map(|config| {
                                let name = &config.name;
                                if config.options.len() > 0 {
                                    let options_str = config.options.join(",");
                                    snippet_completion_item(
                                        name,
                                        format!("{}: ${{1|{}|}};", name, options_str),
                                        CompletionItemKind::PROPERTY,
                                        false,
                                    )
                                } else {
                                    snippet_completion_item(
                                        name,
                                        format!("{}: $0;", name),
                                        CompletionItemKind::PROPERTY,
                                        false,
                                    )
                                }
                            })
                            .collect();
                        Some(CompletionList {
                            is_incomplete: false,
                            items,
                        })
                    }
                    _ => None,
                },
                TokenStaticStyleValuePart::SimplePropertyValue(_, name)
                | TokenStaticStyleValuePart::IncompletePropertyValue(_, name) => {
                    if let Some(config) = backend_config.style_property.iter().find(|x| x.name == name) {
                        let mut items = vec![];
                        for option in config.options.iter() {
                            items.push(simple_completion_item(
                                option,
                                CompletionItemKind::VALUE,
                                false,
                            ));
                        }
                        Some(CompletionList {
                            is_incomplete: false,
                            items,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        WxmlToken::EventName(_event_name, elem) => {
            let mut items: Vec<CompletionItem> = vec![];
            let has_event = |common: &CommonElementAttributes, name: &str| {
                common
                    .event_bindings
                    .iter()
                    .find(|x| x.name.name.as_str() == name)
                    .is_some()
            };
            let common = match &elem.kind {
                ElementKind::Normal { common, .. } | ElementKind::Slot { common, .. } => {
                    Some(common)
                }
                _ => None,
            };
            if let Some(common) = common {
                let tag_name = match &elem.kind {
                    ElementKind::Normal { tag_name, .. } => Some(tag_name),
                    _ => None,
                };
                if let Some(events) = tag_name.and_then(|x| backend_config.list_events(&x.name)) {
                    for ev in events {
                        if has_event(common, &ev.name) {
                            continue;
                        }
                        items.push(simple_completion_item(
                            &ev.name,
                            CompletionItemKind::VARIABLE,
                            ev.deprecated,
                        ));
                    }
                } else {
                    let events = backend_config.list_global_events();
                    for ev in events {
                        if has_event(common, &ev.name) {
                            continue;
                        }
                        items.push(simple_completion_item(
                            &ev.name,
                            CompletionItemKind::VARIABLE,
                            ev.deprecated,
                        ));
                    }
                };
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        WxmlToken::AttributeStaticValue(_loc, _value, name, elem) => {
            if let ElementKind::Normal { tag_name, .. } = &elem.kind {
                let value_options1 = backend_config
                    .search_property(&tag_name.name, &name.name)
                    .map(|x| &x.value_option);
                let value_options2 = backend_config
                    .search_attribute(&tag_name.name, &name.name)
                    .map(|x| &x.value_option);
                if let Some(value_options) = value_options1.or(value_options2) {
                    let list = value_options
                        .iter()
                        .map(|x| {
                            simple_completion_item(
                                &x.value,
                                CompletionItemKind::ENUM_MEMBER,
                                x.deprecated,
                            )
                        })
                        .collect();
                    Some(CompletionList {
                        is_incomplete: false,
                        items: list,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
        WxmlToken::TemplateRef(_name, _loc) => {
            let mut items: Vec<CompletionItem> = vec![];
            if let Some(choices) = project.get_wxml_template_names(abs_path) {
                if choices.len() > 0 {
                    let choices = choices.join(",");
                    items.push(snippet_completion_item(
                        "template is",
                        format!(
                            "template is=\"${{1|{}|}}\" data=\"{{{{ $0 }}}}\" />",
                            choices
                        ),
                        CompletionItemKind::KEYWORD,
                        false,
                    ));
                }
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        // Token::ScriptContent(..) => {
        //     // TODO pass to wxs ls
        // }
        _ => None,
    }
}

fn completion_wxss(
    project: &mut Project,
    backend_config: &BackendConfig,
    abs_path: &Path,
    pos: lsp_types::Position,
    _trigger: &str,
) -> Option<CompletionList> {
    let template = project.get_style_sheet(abs_path).ok()?;
    let token = crate::wxss_utils::find_token_in_position(
        template,
        Position {
            line: pos.line,
            utf16_col: pos.character,
        },
    );
    match token {
        WxssToken::StyleRuleUnknownIdent(_) => {
            let mut items: Vec<CompletionItem> = vec![];
            for config in backend_config.style_property.iter() {
                let name = config.name.as_str();
                if config.options.len() > 0 {
                    let options_str = config.options.join(",");
                    items.push(snippet_completion_item(
                        name,
                        format!("{}: ${{1|{}|}};", name, options_str),
                        CompletionItemKind::PROPERTY,
                        false,
                    ));
                } else {
                    items.push(snippet_completion_item(
                        name,
                        format!("{}: $0;", name),
                        CompletionItemKind::PROPERTY,
                        false,
                    ));
                }
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        WxssToken::IncompletePropertyValue(name) | WxssToken::SimplePropertyValue(_, name) => {
            if let Some(config) = backend_config.style_property.iter().find(|x| x.name == name.content) {
                if config.options.len() > 0 {
                    let mut items: Vec<CompletionItem> = vec![];
                    for option in config.options.iter() {
                        items.push(simple_completion_item(
                            option,
                            CompletionItemKind::PROPERTY,
                            false,
                        ));
                    }
                    Some(CompletionList { is_incomplete: false, items })
                } else {
                    None
                }
            } else {
                None
            }
        }
        WxssToken::PropertyName(_) => {
            let mut items: Vec<CompletionItem> = vec![];
            for config in backend_config.style_property.iter() {
                let name = config.name.as_str();
                items.push(simple_completion_item(
                    name,
                    CompletionItemKind::PROPERTY,
                    false,
                ));
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        WxssToken::MediaQueryUnknownParen(_) | WxssToken::MediaFeatureName(_) => {
            let mut items: Vec<CompletionItem> = vec![];
            for config in backend_config.media_feature.iter() {
                let mut handle_item = |name: &str, has_value: bool| {
                    if config.options.len() > 0 {
                        let options_str = config.options.join(",");
                        items.push(snippet_completion_item(
                            name,
                            format!("{}: ${{1|{}|}}", name, options_str),
                            CompletionItemKind::PROPERTY,
                            false,
                        ));
                    } else if has_value {
                        items.push(snippet_completion_item(
                            name,
                            format!("{}: $0", name),
                            CompletionItemKind::PROPERTY,
                            false,
                        ));
                    } else {
                        items.push(simple_completion_item(
                            name,
                            CompletionItemKind::PROPERTY,
                            false,
                        ));
                    }
                };
                let name = config.name.as_str();
                if config.ty == MediaFeatureType::Range {
                    handle_item(name, true);
                    handle_item(&format!("min-{}", name), true);
                    handle_item(&format!("max-{}", name), true);
                } else {
                    handle_item(name, false);
                }
            }
            Some(CompletionList {
                is_incomplete: false,
                items,
            })
        }
        WxssToken::IncompleteId(_) | WxssToken::Id(_) => {
            let item_set = collect_ids_in_wxml(project, abs_path);
            if item_set.is_empty() {
                None
            } else {
                let items = item_set
                    .into_iter()
                    .map(|x| simple_completion_item(x, CompletionItemKind::VARIABLE, false))
                    .collect();
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }
        }
        WxssToken::IncompleteClass(_) | WxssToken::Class(_, _) => {
            let item_set = collect_classes_in_wxml(project, abs_path);
            if item_set.is_empty() {
                None
            } else {
                let items = item_set
                    .into_iter()
                    .map(|x| simple_completion_item(x, CompletionItemKind::VARIABLE, false))
                    .collect();
                Some(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }
        }
        WxssToken::IncompletePseudoClass(_) | WxssToken::PseudoClass(_, _) => {
            let item_set = backend_config
                .pseudo_class
                .iter()
                .map(|x| simple_completion_item(&x.name, CompletionItemKind::OPERATOR, false))
                .collect();
            Some(CompletionList {
                is_incomplete: false,
                items: item_set,
            })
        }
        WxssToken::IncompletePseudoElement(_, _) | WxssToken::PseudoElement(_, _, _) => {
            let item_set = backend_config
                .pseudo_element
                .iter()
                .map(|x| simple_completion_item(&x.name, CompletionItemKind::OPERATOR, false))
                .collect();
            Some(CompletionList {
                is_incomplete: false,
                items: item_set,
            })
        }
        _ => None,
    }
}
