use glass_easel_template_compiler::parse::{
    tag::{ElementKind, Node, Script},
    Position, Template,
};
use lsp_types::{FoldingRange, FoldingRangeKind, FoldingRangeParams};

use crate::{
    context::{project::FileContentMetadata, FileLang},
    wxss::StyleSheet,
    ServerContext,
};

pub(crate) async fn folding_range(
    ctx: ServerContext,
    params: FoldingRangeParams,
) -> anyhow::Result<Vec<FoldingRange>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<FoldingRange>> {
                let ranges = if let Some(content) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let template = project.get_wxml_tree(&abs_path)?;
                            collect_wxml_folding_ranges(content, template)
                        }
                        FileLang::Wxss => {
                            let template = project.get_style_sheet(&abs_path, false)?;
                            collect_wxss_folding_ranges(template)
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}

fn convert_folding_range(
    loc: std::ops::Range<Position>,
    kind: Option<FoldingRangeKind>,
) -> FoldingRange {
    FoldingRange {
        start_line: loc.start.line,
        start_character: Some(loc.start.utf16_col),
        end_line: loc.end.line,
        end_character: Some(loc.end.utf16_col),
        kind,
        collapsed_text: None,
    }
}

fn collect_wxml_folding_ranges(
    content: &FileContentMetadata,
    template: &Template,
) -> Vec<FoldingRange> {
    let mut ranges = vec![];
    fn collect_in_nodes(ranges: &mut Vec<FoldingRange>, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Text(..) => {}
                Node::Element(elem) => {
                    if let Some(end_loc) = elem.tag_location.end.as_ref() {
                        ranges.push(convert_folding_range(
                            elem.tag_location.start.1.end..end_loc.0.start,
                            None,
                        ));
                    }
                    match &elem.kind {
                        ElementKind::Normal { children, .. }
                        | ElementKind::Pure { children, .. }
                        | ElementKind::For { children, .. } => {
                            collect_in_nodes(ranges, &children);
                        }
                        ElementKind::If {
                            branches,
                            else_branch,
                            ..
                        } => {
                            for br in branches {
                                collect_in_nodes(ranges, &br.2);
                            }
                            if let Some(br) = else_branch {
                                collect_in_nodes(ranges, &br.1);
                            }
                        }
                        ElementKind::Slot { .. }
                        | ElementKind::TemplateRef { .. }
                        | ElementKind::Include { .. } => {}
                        _ => {}
                    }
                }
                Node::Comment(x) => {
                    let mut loc = x.location.clone();
                    loc.start.utf16_col += 3;
                    if loc.end.utf16_col >= 3 {
                        loc.end.utf16_col -= 3;
                    }
                    ranges.push(convert_folding_range(loc, Some(FoldingRangeKind::Comment)));
                }
                Node::UnknownMetaTag(..) => {}
                _ => {}
            }
        }
    }
    for script in template.globals.scripts.iter() {
        match script {
            Script::Inline {
                tag_location,
                content: src,
                content_location,
                ..
            } => {
                if let Some(end_loc) = tag_location.end.as_ref() {
                    let loc = tag_location.start.1.end..end_loc.0.start;
                    ranges.push(convert_folding_range(loc, None));
                    crate::wxs::ScriptMeta::parse(src).collect_folding_ranges(
                        content_location.clone(),
                        content,
                        |loc, option| {
                            ranges.push(convert_folding_range(loc, option));
                        },
                    );
                }
            }
            _ => {}
        }
    }
    for sub in template.globals.sub_templates.iter() {
        if let Some(end_loc) = sub.tag_location.end.as_ref() {
            let loc = sub.tag_location.start.1.end..end_loc.0.start;
            ranges.push(convert_folding_range(loc, None));
        }
        collect_in_nodes(&mut ranges, &sub.content);
    }
    collect_in_nodes(&mut ranges, &template.content);
    ranges
}

fn collect_wxss_folding_ranges(sheet: &StyleSheet) -> Vec<FoldingRange> {
    let mut ranges =
        Vec::with_capacity(sheet.comments.len() + sheet.special_locations.braces.len());
    for comment in sheet.comments.iter() {
        ranges.push(convert_folding_range(
            comment.location.clone(),
            Some(FoldingRangeKind::Comment),
        ));
    }
    for loc in sheet.special_locations.braces.iter() {
        ranges.push(convert_folding_range(loc.clone(), None));
    }
    ranges
}
