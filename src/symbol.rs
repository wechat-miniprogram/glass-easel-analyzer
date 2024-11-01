use lsp_types::{DocumentSymbol, DocumentSymbolParams, Position, Range, SymbolKind};

use crate::ServerContext;

pub(crate) async fn document_symbol(ctx: ServerContext, params: DocumentSymbolParams) -> anyhow::Result<Vec<DocumentSymbol>> {
    let ret = vec![
        // TODO
        DocumentSymbol {
            name: "TEST".to_string(),
            detail: Some("TEST DETAIL".to_string()),
            kind: SymbolKind::NUMBER,
            tags: Default::default(),
            deprecated: Default::default(),
            range: Range {
                start: Position { line: 2, character: 2 },
                end: Position { line: 2, character: 10 },
            },
            selection_range: Range {
                start: Position { line: 2, character: 3 },
                end: Position { line: 2, character: 8 },
            },
            children: Default::default(),
        },
    ];
    Ok(ret)
}
