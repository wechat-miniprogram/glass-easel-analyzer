use glass_easel_template_compiler::parse::{Template, TemplateStructure};
use lsp_types::{DocumentSymbol, DocumentSymbolParams, SymbolKind};

use crate::{
    context::FileLang,
    utils::location_to_lsp_range,
    wxss::{
        keyframe::Keyframe, token::BraceOrSemicolon, CSSParse, List, Rule, RuleOrProperty,
        StyleSheet,
    },
    ServerContext,
};

pub(crate) async fn document_symbol(
    ctx: ServerContext,
    params: DocumentSymbolParams,
) -> anyhow::Result<Vec<DocumentSymbol>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<DocumentSymbol>> {
                let ranges = match file_lang {
                    FileLang::Wxml => {
                        let template = project.get_wxml_tree(&abs_path)?;
                        collect_wxml_symbol_list(template)
                    }
                    FileLang::Wxss => {
                        let template = project.get_style_sheet(&abs_path)?;
                        collect_wxss_symbol_list(template)
                    }
                    _ => vec![],
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}

fn collect_wxml_symbol_list(template: &Template) -> Vec<DocumentSymbol> {
    let mut ret = vec![];
    for sub in &template.globals.sub_templates {
        if sub.name.is("") {
            continue;
        }
        let name_loc = sub.name.location();
        let tag_start_pos = sub.tag_location.start.0.start.clone();
        let tag_end_pos = sub
            .tag_location
            .end
            .as_ref()
            .unwrap_or(&sub.tag_location.start)
            .1
            .end
            .clone();
        #[allow(deprecated)]
        ret.push(DocumentSymbol {
            name: sub.name.name.to_string(),
            detail: Some(format!("<template name={:?}>", sub.name.name)),
            kind: SymbolKind::NAMESPACE,
            tags: Default::default(),
            deprecated: Default::default(),
            selection_range: location_to_lsp_range(&name_loc),
            range: location_to_lsp_range(&(tag_start_pos..tag_end_pos)),
            children: None,
        });
    }
    ret
}

fn collect_wxss_symbol_list(sheet: &StyleSheet) -> Vec<DocumentSymbol> {
    fn rec(rule: &Rule) -> Option<DocumentSymbol> {
        match rule {
            Rule::Unknown(_) => None,
            Rule::Style(x) => {
                let children = convert_option_rule_or_property(&x.brace);
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: x.selector_str.clone(),
                    detail: None,
                    kind: SymbolKind::CLASS,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&x.selector.location()),
                    range: location_to_lsp_range(&x.location()),
                    children,
                })
            }
            Rule::Import(x) =>
            {
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: "@import".to_string(),
                    detail: None,
                    kind: SymbolKind::INTERFACE,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&x.at_import.location()),
                    range: location_to_lsp_range(&x.location()),
                    children: None,
                })
            }
            Rule::Media(x) => {
                let children = convert_option_rule(&x.body);
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: format!("@media {}", x.list_str),
                    detail: None,
                    kind: SymbolKind::MODULE,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&x.at_media.location()),
                    range: location_to_lsp_range(&x.location()),
                    children,
                })
            }
            Rule::FontFace(x) =>
            {
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: "@font-face".to_string(),
                    detail: None,
                    kind: SymbolKind::MODULE,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&x.at_font_face.location()),
                    range: location_to_lsp_range(&x.location()),
                    children: None,
                })
            }
            Rule::Keyframes(x) => {
                let children = match &x.body {
                    Some(BraceOrSemicolon::Brace(x)) => {
                        let children = x
                            .children
                            .iter()
                            .filter_map(|keyframe| {
                                let (name, loc) = match keyframe {
                                    Keyframe::Named { progress, body: _ } => {
                                        let name = progress.known()?;
                                        (name.content.to_string(), name.location())
                                    }
                                    Keyframe::Percentage { progress, body: _ } => {
                                        let progress = progress.known()?;
                                        let v = format!("{}%", progress.value * 100.);
                                        (v, progress.location())
                                    }
                                    Keyframe::Unknown(..) => {
                                        return None;
                                    }
                                };
                                let children = match keyframe {
                                    Keyframe::Named { progress: _, body }
                                    | Keyframe::Percentage { progress: _, body } => {
                                        convert_option_rule_or_property(&body)
                                    }
                                    Keyframe::Unknown(..) => unreachable!(),
                                };
                                #[allow(deprecated)]
                                Some(DocumentSymbol {
                                    name,
                                    detail: None,
                                    kind: SymbolKind::MODULE,
                                    tags: Default::default(),
                                    deprecated: Default::default(),
                                    selection_range: location_to_lsp_range(&loc),
                                    range: location_to_lsp_range(&x.location()),
                                    children,
                                })
                            })
                            .collect();
                        Some(children)
                    }
                    _ => None,
                };
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: format!(
                        "@keyframes {}",
                        x.name.known().map(|x| x.content.as_str()).unwrap_or("")
                    ),
                    detail: None,
                    kind: SymbolKind::MODULE,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&x.at_keyframes.location()),
                    range: location_to_lsp_range(&x.location()),
                    children,
                })
            }
            Rule::UnknownAtRule(kw, x) =>
            {
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: format!("@{}", kw.content),
                    detail: None,
                    kind: SymbolKind::MODULE,
                    tags: Default::default(),
                    deprecated: Default::default(),
                    selection_range: location_to_lsp_range(&kw.location()),
                    range: location_to_lsp_range(&(kw.location().start..x.location().end)),
                    children: None,
                })
            }
        }
    }
    fn convert_option_rule(
        x: &Option<BraceOrSemicolon<List<Rule>>>,
    ) -> Option<Vec<DocumentSymbol>> {
        match x {
            Some(BraceOrSemicolon::Brace(x)) => {
                let children = x.children.iter().filter_map(rec).collect();
                Some(children)
            }
            _ => None,
        }
    }
    fn convert_option_rule_or_property(
        x: &Option<BraceOrSemicolon<List<RuleOrProperty>>>,
    ) -> Option<Vec<DocumentSymbol>> {
        match x {
            Some(BraceOrSemicolon::Brace(x)) => {
                let children = x
                    .children
                    .iter()
                    .filter_map(|rp| match rp {
                        RuleOrProperty::Rule(rule) => rec(rule),
                        RuleOrProperty::Property(_) => None,
                    })
                    .collect();
                Some(children)
            }
            _ => None,
        }
    }
    sheet.items.iter().filter_map(rec).collect()
}
