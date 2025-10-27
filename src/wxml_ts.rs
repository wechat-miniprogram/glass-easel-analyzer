use lsp_types::{Position, Range, Url};
use serde::{Deserialize, Serialize};

use crate::{
    context::FileLang,
    utils::{location_to_lsp_range, lsp_range_to_location},
    ServerContext,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprReleaseParams {
    text_document_uri: Url,
}

pub(crate) async fn tmpl_converted_expr_release(
    ctx: ServerContext,
    params: TmplConvertedExprReleaseParams,
) -> anyhow::Result<bool> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_uri,
            move |project, abs_path, file_lang| -> anyhow::Result<bool> {
                let success = if let Some(_) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => project.wxml_converted_expr_release(&abs_path),
                        _ => false,
                    }
                } else {
                    false
                };
                Ok(success)
            },
        )
        .await??;
    Ok(ret)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprCodeParams {
    text_document_uri: Url,
    ts_env: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprCode {
    code: String,
}

pub(crate) async fn tmpl_converted_expr_code(
    ctx: ServerContext,
    params: TmplConvertedExprCodeParams,
) -> anyhow::Result<Option<TmplConvertedExprCode>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Option<TmplConvertedExprCode>> {
                let code = if let Some(_) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let code =
                                project.wxml_converted_expr_code(&abs_path, &params.ts_env)?;
                            Some(TmplConvertedExprCode { code })
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                Ok(code)
            },
        )
        .await??;
    Ok(ret)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprGetSourceLocationParams {
    text_document_uri: Url,
    loc: Range,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprGetSourceLocation {
    src: Range,
}

pub(crate) async fn tmpl_converted_expr_get_source_location(
    ctx: ServerContext,
    params: TmplConvertedExprGetSourceLocationParams,
) -> anyhow::Result<Option<TmplConvertedExprGetSourceLocation>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_uri,
            move |project,
                  abs_path,
                  file_lang|
                  -> anyhow::Result<Option<TmplConvertedExprGetSourceLocation>> {
                let src_loc = if let Some(_) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let loc = lsp_range_to_location(&params.loc);
                            project.wxml_converted_expr_get_source_location(&abs_path, loc)
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                let src = src_loc.as_ref().map(|x| location_to_lsp_range(x));
                Ok(src.map(|src| TmplConvertedExprGetSourceLocation { src }))
            },
        )
        .await??;
    Ok(ret)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprGetTokenAtSourcePositionParams {
    text_document_uri: Url,
    pos: Position,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TmplConvertedExprGetTokenAtSourcePosition {
    src: Range,
    dest: Position,
}

pub(crate) async fn tmpl_converted_expr_get_token_at_source_position(
    ctx: ServerContext,
    params: TmplConvertedExprGetTokenAtSourcePositionParams,
) -> anyhow::Result<Option<TmplConvertedExprGetTokenAtSourcePosition>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document_uri,
            move |project,
                  abs_path,
                  file_lang|
                  -> anyhow::Result<Option<TmplConvertedExprGetTokenAtSourcePosition>> {
                let ret = if let Some(_) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let pos = crate::wxss::Position {
                                line: params.pos.line,
                                utf16_col: params.pos.character,
                            };
                            project.wxml_converted_expr_get_token_at_source_position(&abs_path, pos)
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                let ret = ret.map(|(src, dest)| TmplConvertedExprGetTokenAtSourcePosition {
                    src: location_to_lsp_range(&src),
                    dest: Position {
                        line: dest.line,
                        character: dest.utf16_col,
                    },
                });
                Ok(ret)
            },
        )
        .await??;
    Ok(ret)
}
