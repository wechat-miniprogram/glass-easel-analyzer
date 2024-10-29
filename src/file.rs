use lsp_types::DidOpenTextDocumentParams;

pub(crate) async fn did_open(params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File opened: {}", params.text_document.uri.as_str());
    Ok(())
}
