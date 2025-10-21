use glass_easel_template_compiler::stringify::{Stringifier, StringifyOptions};
use lsp_types::{DocumentFormattingParams, TextEdit};

use crate::{context::FileLang, ServerContext};

pub(crate) async fn formatting(
    ctx: ServerContext,
    params: DocumentFormattingParams,
) -> anyhow::Result<Vec<TextEdit>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<TextEdit>> {
                let ret = match file_lang {
                    FileLang::Wxml => {
                        let template = project.get_wxml_tree(&abs_path)?;
                        let options = StringifyOptions {
                            tab_size: params.options.tab_size,
                            use_tab_character: !params.options.insert_spaces,
                            ..Default::default()
                        };
                        let mut out = String::new();
                        Stringifier::new(&mut out, "", None, options).run(template)?;
                        let text_edit = TextEdit {
                            range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: lsp_types::Position {
                                    line: u32::MAX,
                                    character: u32::MAX,
                                },
                            },
                            new_text: out,
                        };
                        vec![text_edit]
                    }
                    _ => vec![],
                };
                Ok(ret)
            },
        )
        .await??;
    Ok(ret)
}
