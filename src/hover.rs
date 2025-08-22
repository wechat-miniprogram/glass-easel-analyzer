use std::{ops::Range, path::Path};

use glass_easel_template_compiler::parse::{tag::ElementKind, Position};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Url};

use crate::{
    context::{backend_configuration::*, project::Project, FileLang},
    utils::location_to_lsp_range,
    wxml_utils::{ScopeKind, Token as WxmlToken, TokenStaticStyleValuePart},
    wxss::CSSParse,
    wxss_utils::Token as WxssToken,
    BackendConfig, ServerContext,
};

pub(crate) async fn hover(
    ctx: ServerContext,
    params: HoverParams,
) -> anyhow::Result<Option<Hover>> {
    let backend_config = ctx.backend_config();
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_position_params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Option<Hover>> {
                let hover = match file_lang {
                    FileLang::Wxml => hover_wxml(
                        project,
                        &backend_config,
                        &abs_path,
                        params.text_document_position_params.position,
                    ),
                    FileLang::Wxss => hover_wxss(
                        project,
                        &backend_config,
                        &abs_path,
                        params.text_document_position_params.position,
                    ),
                    _ => None,
                };
                Ok(hover)
            },
        )
        .await??;
    Ok(ret)
}

fn plain_str_hover_contents(s: impl Into<String>) -> HoverContents {
    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::PlainText,
        value: s.into(),
    })
}

fn md_str_hover_contents(s: impl Into<String>) -> HoverContents {
    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: s.into(),
    })
}

fn reference_args_str(reference: &Option<Url>) -> String {
    if let Some(r) = reference {
        format!("\n\n[Reference]({})", r)
    } else {
        format!("")
    }
}

fn property_name_hint(backend_config: &BackendConfig, name: &str, loc: Range<Position>) -> Option<Hover> {
    backend_config
        .style_property
        .iter()
        .find(|config| config.name == name)
        .map(|config| {
            let StylePropertyConfig {
                name,
                options: _,
                description,
                reference,
            } = config;
            let contents = md_str_hover_contents(format!(
                "**{}** *property*\n\n{}{}",
                name,
                description,
                reference_args_str(reference)
            ));
            Hover {
                contents,
                range: Some(location_to_lsp_range(&loc)),
            }
        })
}

fn hover_wxml(
    project: &mut Project,
    backend_config: &BackendConfig,
    abs_path: &Path,
    pos: lsp_types::Position,
) -> Option<Hover> {
    let template = project.get_wxml_tree(abs_path).ok()?;
    let token = crate::wxml_utils::find_token_in_position(
        template,
        Position {
            line: pos.line,
            utf16_col: pos.character,
        },
    );
    match token {
        WxmlToken::ScopeRef(loc, kind) => {
            let contents = match kind {
                ScopeKind::Script(_) => plain_str_hover_contents("wxs script"),
                ScopeKind::ForScope(_, _) => plain_str_hover_contents("wx:for scope"),
                ScopeKind::SlotValue(_, _) => plain_str_hover_contents("slot value"),
                ScopeKind::LetVar(_, _) => plain_str_hover_contents("variable"),
            };
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&loc)),
            })
        }
        WxmlToken::TagName(tag_name) => {
            let contents = if let Some(_target_path) =
                project.get_target_component_path(abs_path, &tag_name.name)
            {
                plain_str_hover_contents("custom component")
            } else if let Some(elem) = backend_config.search_component(&tag_name.name) {
                let ComponentConfig {
                    tag_name,
                    description,
                    reference,
                    ..
                } = elem;
                md_str_hover_contents(format!(
                    "**{}** *component*\n\n{}{}",
                    tag_name,
                    description,
                    reference_args_str(reference)
                ))
            } else if let Some(elem) = backend_config.search_element(&tag_name.name) {
                let ElementConfig {
                    tag_name,
                    description,
                    reference,
                    ..
                } = elem;
                md_str_hover_contents(format!(
                    "**{}** *element*\n\n{}{}",
                    tag_name,
                    description,
                    reference_args_str(reference)
                ))
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&tag_name.location)),
            })
        }
        WxmlToken::StaticStylePropertyName(x, _) => {
            property_name_hint(backend_config, &x.name, x.location.clone())
        }
        WxmlToken::StaticStyleValuePart(part, _) => {
            match part {
                TokenStaticStyleValuePart::PropertyName(loc, name) => {
                    property_name_hint(backend_config, &name, loc.clone())
                }
                _ => None,
            }
        }
        WxmlToken::AttributeName(attr_name, elem) => {
            let tag_name = match &elem.kind {
                ElementKind::Normal { tag_name, .. } => Some(tag_name),
                _ => None,
            };
            let contents = if let Some(tag_name) = tag_name {
                if let Some(_target_path) =
                    project.get_target_component_path(abs_path, &tag_name.name)
                {
                    plain_str_hover_contents("custom component property")
                } else if let Some(prop) =
                    backend_config.search_property(&tag_name.name, &attr_name.name)
                {
                    let PropertyConfig {
                        name,
                        ty,
                        description,
                        reference,
                        ..
                    } = prop;
                    let ty_args = if ty.len() > 0 {
                        format!(": {}", ty)
                    } else {
                        format!("")
                    };
                    md_str_hover_contents(format!(
                        "**{}**{} *property*\n\n{}{}",
                        name,
                        ty_args,
                        description,
                        reference_args_str(reference)
                    ))
                } else if let Some(attr) =
                    backend_config.search_attribute(&tag_name.name, &attr_name.name)
                {
                    let AttributeConfig {
                        name,
                        description,
                        reference,
                        ..
                    } = attr;
                    md_str_hover_contents(format!(
                        "**{}** *attribute*\n\n{}{}",
                        name,
                        description,
                        reference_args_str(reference)
                    ))
                } else {
                    plain_str_hover_contents("unknown")
                }
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&attr_name.location)),
            })
        }
        WxmlToken::EventName(event_name, elem) => {
            let tag_name = match &elem.kind {
                ElementKind::Normal { tag_name, .. } => Some(tag_name),
                _ => None,
            };
            let contents = if let Some(tag_name) = tag_name {
                if let Some(_target_path) =
                    project.get_target_component_path(abs_path, &tag_name.name)
                {
                    plain_str_hover_contents("custom component event")
                } else if let Some(ev) =
                    backend_config.search_event(&tag_name.name, &event_name.name)
                {
                    let EventConfig {
                        name,
                        description,
                        reference,
                        ..
                    } = ev;
                    md_str_hover_contents(format!(
                        "**{}** *event*\n\n{}{}",
                        name,
                        description,
                        reference_args_str(reference)
                    ))
                } else {
                    plain_str_hover_contents("unknown")
                }
            } else if let Some(ev) = backend_config.search_global_event(&event_name.name) {
                let EventConfig {
                    name,
                    description,
                    reference,
                    ..
                } = ev;
                md_str_hover_contents(format!(
                    "**{}** *event*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ))
            } else {
                plain_str_hover_contents("unknown")
            };
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&event_name.location)),
            })
        }
        WxmlToken::AttributeStaticValue(loc, value, name, elem) => {
            if let ElementKind::Normal { tag_name, .. } = &elem.kind {
                let value_options1 = backend_config
                    .search_property(&tag_name.name, &name.name)
                    .map(|x| (&x.value_option, &x.reference));
                let value_options2 = backend_config
                    .search_attribute(&tag_name.name, &name.name)
                    .map(|x| (&x.value_option, &x.reference));
                if let Some((value_options, reference)) = value_options1.or(value_options2) {
                    if let Some(option) = value_options.iter().find(|x| x.value.as_str() == value) {
                        let contents = md_str_hover_contents(format!(
                            "**{}**\n\n{}{}",
                            option.value,
                            option.description,
                            reference_args_str(reference)
                        ));
                        Some(Hover {
                            contents,
                            range: Some(location_to_lsp_range(&loc)),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn hover_wxss(
    project: &mut Project,
    backend_config: &BackendConfig,
    abs_path: &Path,
    pos: lsp_types::Position,
) -> Option<Hover> {
    let sheet = project.get_style_sheet(abs_path, false).ok()?;
    let token = crate::wxss_utils::find_token_in_position(
        sheet,
        Position {
            line: pos.line,
            utf16_col: pos.character,
        },
    );
    match token {
        WxssToken::TagName(x) => {
            let contents = md_str_hover_contents(format!(
                r#"Tag name selector `{s}`, matches `<{s} ...>`."#,
                s = x.content
            ));
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&x.location())),
            })
        }
        WxssToken::Id(x) => {
            let contents = md_str_hover_contents(format!(
                r#"ID selector `#{s}`, matches `<... id="{s}">`."#,
                s = x.content
            ));
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(&x.location())),
            })
        }
        WxssToken::Class(op, x) => {
            let contents = md_str_hover_contents(format!(
                r#"Class selector `.{s}`, matches `<... class="{s}">`."#,
                s = x.content
            ));
            Some(Hover {
                contents,
                range: Some(location_to_lsp_range(
                    &(op.location().start..x.location().end),
                )),
            })
        }
        WxssToken::PseudoClass(op, x) => backend_config
            .pseudo_class
            .iter()
            .find(|config| config.name == x.name())
            .map(|config| {
                let PseudoClassConfig {
                    name,
                    description,
                    reference,
                } = config;
                let contents = md_str_hover_contents(format!(
                    "**{}** *pseudo class*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ));
                let loc = op.location().start..x.location().end;
                Hover {
                    contents,
                    range: Some(location_to_lsp_range(&loc)),
                }
            }),
        WxssToken::PseudoElement(op, _, x) => backend_config
            .pseudo_element
            .iter()
            .find(|config| config.name == x.name())
            .map(|config| {
                let PseudoElementConfig {
                    name,
                    description,
                    reference,
                } = config;
                let contents = md_str_hover_contents(format!(
                    "**{}** *pseudo element*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ));
                let loc = op.location().start..x.location().end;
                Hover {
                    contents,
                    range: Some(location_to_lsp_range(&loc)),
                }
            }),
        WxssToken::PropertyName(x) => backend_config
            .style_property
            .iter()
            .find(|config| config.name == x.content)
            .map(|config| {
                let StylePropertyConfig {
                    name,
                    options: _,
                    description,
                    reference,
                } = config;
                let contents = md_str_hover_contents(format!(
                    "**{}** *property*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ));
                Hover {
                    contents,
                    range: Some(location_to_lsp_range(&x.location())),
                }
            }),
        WxssToken::MediaType(x) => backend_config
            .media_type
            .iter()
            .find(|config| config.name == x.content)
            .map(|config| {
                let MediaTypeConfig {
                    name,
                    description,
                    reference,
                } = config;
                let contents = md_str_hover_contents(format!(
                    "**{}** *media type*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ));
                Hover {
                    contents,
                    range: Some(location_to_lsp_range(&x.location())),
                }
            }),
        WxssToken::MediaFeatureName(x) => backend_config
            .media_feature
            .iter()
            .find(|config| {
                if config.name == x.content {
                    return true;
                }
                if config.ty == MediaFeatureType::Range {
                    if let Some(x) = x.content.strip_prefix("min-") {
                        if config.name == x {
                            return true;
                        }
                    }
                    if let Some(x) = x.content.strip_prefix("max-") {
                        if config.name == x {
                            return true;
                        }
                    }
                }
                false
            })
            .map(|config| {
                let MediaFeatureConfig {
                    name,
                    ty: _,
                    options: _,
                    description,
                    reference,
                } = config;
                let contents = md_str_hover_contents(format!(
                    "**{}** *media feature*\n\n{}{}",
                    name,
                    description,
                    reference_args_str(reference)
                ));
                Hover {
                    contents,
                    range: Some(location_to_lsp_range(&x.location())),
                }
            }),
        _ => None,
    }
}
