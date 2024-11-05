use glass_easel_template_compiler::parse::{tag::{ElementKind, Node}, Position, Template};
use lsp_types::{FoldingRange, FoldingRangeKind, FoldingRangeParams};

use crate::ServerContext;

pub(crate) async fn folding_range(ctx: ServerContext, params: FoldingRangeParams) -> anyhow::Result<Vec<FoldingRange>> {
    let ret = ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| -> anyhow::Result<Vec<FoldingRange>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                let template = project.get_wxml_tree(&abs_path)?;
                collect_wxml_folding_ranges(template)
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

fn collect_wxml_folding_ranges(template: &Template) -> Vec<FoldingRange> {
    let mut ranges = vec![];
    fn convert_folding_range(loc: std::ops::Range<Position>, kind: Option<FoldingRangeKind>) -> FoldingRange {
        FoldingRange {
            start_line: loc.start.line,
            start_character: Some(loc.start.utf16_col),
            end_line: loc.end.line,
            end_character: Some(loc.end.utf16_col),
            kind,
            collapsed_text: None,
        }
    }
    fn collect_in_nodes(ranges: &mut Vec<FoldingRange>, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Text(..) => {}
                Node::Element(elem) => {
                    if let Some(end_loc) = elem.tag_location.end.as_ref() {
                        ranges.push(convert_folding_range(elem.tag_location.start.1.end..end_loc.0.start, None));
                    }
                    match &elem.kind {
                        ElementKind::Normal { children, .. } |
                        ElementKind::Pure { children, .. } |
                        ElementKind::For { children, .. } => {
                            collect_in_nodes(ranges, &children);
                        }
                        ElementKind::If { branches, else_branch } => {
                            for br in branches {
                                collect_in_nodes(ranges, &br.2);
                            }
                            if let Some(br) = else_branch {
                                collect_in_nodes(ranges, &br.1);
                            }
                        }
                        ElementKind::Slot { .. } |
                        ElementKind::TemplateRef { .. } |
                        ElementKind::Include { .. } => {}
                    }
                }
                Node::Comment(_, loc) => {
                    let mut loc = loc.clone();
                    loc.start.utf16_col += 3;
                    if loc.end.utf16_col >= 3 { loc.end.utf16_col -= 3; }
                    ranges.push(convert_folding_range(loc, Some(FoldingRangeKind::Comment)));
                }
                Node::UnknownMetaTag(..) => {}
            }
        }
    }
    for (tag_location, _, _, nodes) in template.globals.sub_templates.iter() {
        if let Some(end_loc) = tag_location.end.as_ref() {
            let loc = tag_location.start.1.end..end_loc.0.start;
            ranges.push(convert_folding_range(loc, None));
        }
        collect_in_nodes(&mut ranges, nodes);
    }
    collect_in_nodes(&mut ranges, &template.content);
    ranges
}
