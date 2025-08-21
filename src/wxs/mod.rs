use std::ops::Range;

use glass_easel_template_compiler::parse::Position;
use swc_common::{comments::SingleThreadedComments, BytePos};
use swc_ecma_lexer::{token::{TokenKind, WordKind}, Lexer, StringInput};

use crate::{context::project::FileContentMetadata, semantic::TokenType};

fn span_to_location(
    file_content_metadata: &FileContentMetadata,
    span: swc_common::Span,
    offset: usize,
) -> Range<Position> {
    let (start_line, start_utf16_col) = file_content_metadata
        .line_utf16_col_for_content_index(span.lo.0 as usize + offset);
    let (end_line, end_utf16_col) = file_content_metadata
        .line_utf16_col_for_content_index(span.hi.0 as usize + offset);
    Position { line: start_line, utf16_col: start_utf16_col }..Position { line: end_line, utf16_col: end_utf16_col }
}

pub(crate) struct ScriptMeta {}

impl ScriptMeta {
    pub(crate) fn parse(
        src: &str,
        content_location: Range<Position>,
        file_content_metadata: &FileContentMetadata,
        mut f: impl FnMut(TokenType, Range<Position>, bool),
    ) {
        let src_byte_offset = file_content_metadata.content_index_for_line_utf16_col(
            content_location.start.line,
            content_location.start.utf16_col,
        );

        // run lexer
        let comments = SingleThreadedComments::default();
        let es_lexer = Lexer::new(
            swc_ecma_lexer::Syntax::Es(swc_ecma_lexer::EsSyntax::default()),
            swc_ecma_ast::EsVersion::EsNext,
            StringInput::new(src, BytePos(0), BytePos(src.len() as u32)),
            Some(&comments),
        );
        let Ok(tokens) = swc_ecma_lexer::lexer(es_lexer) else {
            return;
        };
        let (comments_leading, comments_trailing) = comments.take_all();

        // collect main parts
        let mut after_dot = false;
        for token_and_span in tokens {
            let loc = span_to_location(file_content_metadata, token_and_span.span, src_byte_offset);
            match token_and_span.token.kind() {
                TokenKind::Word(x) => {
                    if after_dot {
                        f(TokenType::Property, loc, false);
                    } else {
                        match x {
                            WordKind::Ident(_) => f(TokenType::Variable, loc, false),
                            _ => f(TokenType::Keyword, loc, false),
                        }
                    }
                }
                TokenKind::Str | TokenKind::Template => f(TokenType::String, loc, false),
                TokenKind::Num | TokenKind::BigInt => f(TokenType::Number, loc, false),
                TokenKind::Regex => {
                    let mut loc = loc;
                    loc.start.utf16_col -= 1;
                    f(TokenType::RegExp, loc, true)
                },
                TokenKind::Arrow
                | TokenKind::Hash
                | TokenKind::At
                | TokenKind::Dot
                | TokenKind::DotDotDot
                | TokenKind::Bang
                | TokenKind::LParen
                | TokenKind::RParen
                | TokenKind::LBracket
                | TokenKind::RBracket
                | TokenKind::LBrace
                | TokenKind::RBrace
                | TokenKind::Semi
                | TokenKind::Comma
                | TokenKind::BackQuote
                | TokenKind::Colon
                | TokenKind::BinOp(_)
                | TokenKind::AssignOp(_)
                | TokenKind::DollarLBrace 
                | TokenKind::QuestionMark
                | TokenKind::PlusPlus
                | TokenKind::MinusMinus
                | TokenKind::Tilde => f(TokenType::Operator, loc, false),
                | TokenKind::Shebang => f(TokenType::Comment, loc, false),
                _ => {}
            }
            after_dot = match token_and_span.token.kind() {
                TokenKind::Dot => true,
                _ => false,
            };
        }

        // collect comments
        for pc in comments_leading.borrow().iter().chain(comments_trailing.borrow().iter()) {
            let (_pos, comments) = pc;
            for comment in comments {
                let loc = span_to_location(file_content_metadata, comment.span, src_byte_offset);
                f(TokenType::Comment, loc, false);
            }
        }
    }
}
