use glass_easel_template_compiler::parse::ParseErrorLevel;
use lsp_types::{Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams, Position, PublishDiagnosticsParams, Range, Uri};

use crate::ServerContext;

async fn update_diagnostic(ctx: &ServerContext, uri: Uri, text: String) -> anyhow::Result<()> {
    let ctx = ctx.clone();
    let ctx2 = ctx.clone();
    ctx.project_thread_task(uri.clone(), move |project, abs_path| -> anyhow::Result<()> {
        let ctx = ctx2;
        let tmpl_path = project.unix_rel_path(&abs_path)?;
        let err_list = project.template_group().add_tmpl(&tmpl_path, &text);
        let diagnostics = err_list.into_iter().map(|x| {
            Diagnostic {
                range: Range {
                    start: Position { line: x.location.start.line, character: x.location.start.utf16_col },
                    end: Position { line: x.location.end.line, character: x.location.end.utf16_col },
                },
                severity: Some(match x.level() {
                    ParseErrorLevel::Fatal => DiagnosticSeverity::ERROR,
                    ParseErrorLevel::Error => DiagnosticSeverity::ERROR,
                    ParseErrorLevel::Warn => DiagnosticSeverity::WARNING,
                    ParseErrorLevel::Note => DiagnosticSeverity::HINT,
                }),
                code: Some(lsp_types::NumberOrString::Number(x.code() as i32)),
                message: x.kind.to_string(),
                ..Default::default()
            }
        }).collect();
        ctx.send_notification("textDocument/publishDiagnostics", PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        })?;
        Ok(())
    }).await?
}

pub(crate) async fn did_open(ctx: &ServerContext, params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
    log::debug!("File opened: {}", params.text_document.uri.as_str());
    if let Err(err) = update_diagnostic(ctx, params.text_document.uri, params.text_document.text).await {
        log::error!("{}", err);
    }
    Ok(())
}
