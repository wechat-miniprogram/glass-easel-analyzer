use glass_easel_template_compiler::parse::{Template, TemplateStructure};
use lsp_types::{DocumentSymbol, DocumentSymbolParams, Position, Range, SymbolKind};

use crate::ServerContext;

pub(crate) async fn document_symbol(ctx: ServerContext, params: DocumentSymbolParams) -> anyhow::Result<Vec<DocumentSymbol>> {
    let ret = ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| -> anyhow::Result<Vec<DocumentSymbol>> {
        let ranges = match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                let template = project.get_wxml_tree(&abs_path)?;
                collect_wxml_symbol_list(template)
            }
            _ => vec![],
        };
        Ok(ranges)
    }).await??;
    Ok(ret)
}

fn collect_wxml_symbol_list(template: &Template) -> Vec<DocumentSymbol> {
    let mut ret = vec![];
    for sub in &template.globals.sub_templates {
        let name_loc = sub.name.location();
        let tag_start_pos = sub.tag_location.start.0.start.clone();
        let tag_end_pos = sub.tag_location.end.as_ref().unwrap_or(&sub.tag_location.start).1.end.clone();
        #[allow(deprecated)]
        ret.push(DocumentSymbol {
            name: sub.name.name.to_string(),
            detail: Some(format!("<template name={:?}>", sub.name.name)),
            kind: SymbolKind::NAMESPACE,
            tags: Default::default(),
            deprecated: Default::default(),
            selection_range: Range {
                start: Position::new(name_loc.start.line, name_loc.start.utf16_col),
                end: Position::new(name_loc.end.line, name_loc.end.utf16_col),
            },
            range: Range {
                start: Position::new(tag_start_pos.line, tag_start_pos.utf16_col),
                end: Position::new(tag_end_pos.line, tag_end_pos.utf16_col),
            },
            children: None,
        });
    }
    ret
}
