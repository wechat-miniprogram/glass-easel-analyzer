use std::cmp::Ordering;

use glass_easel_template_compiler::parse::Position;
use lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensParams,
    SemanticTokensRangeParams,
};

use crate::{
    context::{project::FileContentMetadata, FileLang},
    ServerContext,
};

mod wxml;
mod wxss;

pub(crate) const TOKEN_TYPES: [SemanticTokenType; 12] = [
    SemanticTokenType::TYPE,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::MACRO,
];

// this list MUST matches the list above
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
enum TokenType {
    Type = 0,
    Variable,
    Property,
    Event,
    Function,
    Method,
    Keyword,
    Comment,
    String,
    Number,
    Operator,
    Macro,
}

pub(crate) const TOKEN_MODIFIERS: [SemanticTokenModifier; 3] = [
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::DEPRECATED,
];

// this list MUST matches the list above
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum TokenModifier {
    None = 0x00000000,
    Declaration = 0x00000001,
    Definition = 0x00000002,
    #[allow(dead_code)]
    Deprecated = 0x00000004,
}

pub(crate) async fn tokens_full(
    ctx: ServerContext,
    params: SemanticTokensParams,
) -> anyhow::Result<SemanticTokens> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<_> {
                let data = if let Some(content) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let template = project.get_wxml_tree(&abs_path)?;
                            let range = Position {
                                line: 0,
                                utf16_col: 0,
                            }..Position {
                                line: u32::MAX,
                                utf16_col: u32::MAX,
                            };
                            wxml::find_wxml_semantic_tokens(content, template, range)
                        }
                        FileLang::Wxss => {
                            let sheet = project.get_style_sheet(&abs_path)?;
                            let range = Position {
                                line: 0,
                                utf16_col: 0,
                            }..Position {
                                line: u32::MAX,
                                utf16_col: u32::MAX,
                            };
                            wxss::find_wxss_semantic_tokens(content, sheet, range)
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                Ok(SemanticTokens {
                    result_id: None,
                    data,
                })
            },
        )
        .await??;
    Ok(ret)
}

pub(crate) async fn tokens_range(
    ctx: ServerContext,
    params: SemanticTokensRangeParams,
) -> anyhow::Result<SemanticTokens> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<_> {
                let data = if let Some(content) = project.cached_file_content(&abs_path) {
                    match file_lang {
                        FileLang::Wxml => {
                            let template = project.get_wxml_tree(&abs_path)?;
                            let start = Position {
                                line: params.range.start.line,
                                utf16_col: params.range.start.character,
                            };
                            let end = Position {
                                line: params.range.end.line,
                                utf16_col: params.range.end.character,
                            };
                            wxml::find_wxml_semantic_tokens(content, template, start..end)
                        }
                        FileLang::Wxss => {
                            let sheet = project.get_style_sheet(&abs_path)?;
                            let start = Position {
                                line: params.range.start.line,
                                utf16_col: params.range.start.character,
                            };
                            let end = Position {
                                line: params.range.end.line,
                                utf16_col: params.range.end.character,
                            };
                            wxss::find_wxss_semantic_tokens(content, sheet, start..end)
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                Ok(SemanticTokens {
                    result_id: None,
                    data,
                })
            },
        )
        .await??;
    Ok(ret)
}

struct SemanticTokenGenerator {
    rel_line: u32,
    rel_col: u32,
    range: std::ops::Range<Position>,
    generated: Vec<SemanticToken>,
}

impl SemanticTokenGenerator {
    fn new(range: std::ops::Range<Position>) -> Self {
        Self {
            rel_line: 0,
            rel_col: 0,
            range,
            generated: vec![],
        }
    }

    fn push(
        &mut self,
        content: &FileContentMetadata,
        mut location: std::ops::Range<Position>,
        ty: TokenType,
        modifier: u32,
    ) -> bool {
        if location.start >= self.range.end {
            return false;
        }
        if location.start < self.range.start {
            return true;
        }
        if location.start.line < self.rel_line
            || (location.start.line == self.rel_line && location.start.utf16_col < self.rel_col)
        {
            location.start.line = self.rel_line;
            location.end.line = self.rel_col;
        }
        for line in location.start.line..=location.end.line {
            let start = if line == location.start.line {
                location.start.utf16_col
            } else {
                0
            };
            let end = if line == location.end.line {
                location.end.utf16_col
            } else {
                content.get_line_utf16_len(line)
            };
            let length = end.saturating_sub(start);
            if length == 0 {
                continue;
            }
            let delta_line = line - self.rel_line;
            let delta_start = start - if delta_line > 0 { 0 } else { self.rel_col };
            self.generated.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: ty as u32,
                token_modifiers_bitset: modifier,
            });
            self.rel_line = line;
            self.rel_col = start;
        }
        true
    }

    fn finish(self) -> Vec<SemanticToken> {
        self.generated
    }
}
