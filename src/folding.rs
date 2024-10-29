use lsp_types::{FoldingRange, FoldingRangeKind, FoldingRangeParams};

pub(crate) async fn folding_range(params: FoldingRangeParams) -> anyhow::Result<Vec<FoldingRange>> {
    let ret = vec![
        // TODO
        FoldingRange {
            start_line: 2,
            start_character: None,
            end_line: 5,
            end_character: None,
            kind: Some(FoldingRangeKind::Region),
            collapsed_text: None,
        },
    ];
    Ok(ret)
}
