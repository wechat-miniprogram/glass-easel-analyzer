use crate::wxss::{font_face::FontFaceRule, import::ImportRule, keyframe::{Keyframe, KeyframesRule}, media::{MediaAndKeyword, MediaFeature, MediaOrKeyword, MediaQueryList, MediaRule, MediaType}, property::Property, rule::{IdentOrFunction, Selector, StyleRule}, token::*, CSSParse, List, MaybeUnknown, Rule, RuleOrProperty, StyleSheet};

use super::*;

struct SemanticTokenRet<'a> {
    gen: SemanticTokenGenerator,
    comments: std::iter::Peekable<std::slice::Iter<'a, Comment>>,
    content: &'a FileContentMetadata,
}

impl<'a> SemanticTokenRet<'a> {
    fn new(
        gen: SemanticTokenGenerator,
        comments: &'a [Comment],
        content: &'a FileContentMetadata,
    ) -> Self {
        Self {
            gen,
            comments: comments.iter().peekable(),
            content,
        }
    }

    fn push(
        &mut self,
        location: std::ops::Range<Position>,
        ty: TokenType,
        modifier: u32,
    ) -> bool {
        while let Some(next_comment) = self.comments.peek() {
            if next_comment.location.start > location.start {
                break;
            }
            let next = self.comments.next().unwrap();
            if !self.gen.push(self.content, next.location.clone(), TokenType::Comment, 0) {
                return false;
            }
        }
        self.gen.push(self.content, location, ty, modifier)
    }

    fn finish(mut self) -> Vec<SemanticToken> {
        for next in self.comments {
            if !self.gen.push(self.content, next.location.clone(), TokenType::Comment, 0) {
                break;
            }
        }
        self.gen.finish()
    }
}

pub(super) fn find_wxss_semantic_tokens(content: &FileContentMetadata, sheet: &StyleSheet, range: std::ops::Range<Position>) -> Vec<SemanticToken> {
    let mut gen = SemanticTokenRet::new(
        SemanticTokenGenerator::new(range),
        &sheet.comments,
        content,
    );
    for item in sheet.items.iter() {
        if !find_in_rule(&mut gen, item) {
            break;
        }
    }
    gen.finish()
}

fn find_in_rule(ret: &mut SemanticTokenRet, rule: &Rule) -> bool {
    match rule {
        Rule::Unknown(tt_list) => find_in_token_tree_list(ret, &tt_list),
        Rule::Style(x) => find_in_style_rule(ret, x),
        Rule::Import(x) => find_in_import_rule(ret, x),
        Rule::Media(x) => find_in_media_rule(ret, x),
        Rule::FontFace(x) => find_in_font_face_rule(ret, x),
        Rule::Keyframes(x) => find_in_keyframes_rule(ret, x),
        Rule::UnknownAtRule(kw, tt_list) => {
            ret.push(kw.location(), TokenType::Keyword, 0)
                && find_in_token_tree_list(ret, &tt_list)
        }
    }
}

fn find_in_import_rule(ret: &mut SemanticTokenRet, import: &ImportRule) -> bool {
    let ImportRule { at_import, url, condition, semicolon } = import;
    ret.push(at_import.location(), TokenType::Keyword, 0)
        && find_in_maybe_unknown(ret, url, |ret, x| {
            ret.push(x.location(), TokenType::String, 0)
        })
        && find_in_token_tree_list(ret, condition)
        && find_in_option(ret, semicolon, |ret, x| {
            ret.push(x.location(), TokenType::Operator, 0)
        })
}

fn find_in_media_rule(ret: &mut SemanticTokenRet, media: &MediaRule) -> bool {
    let MediaRule { at_media, list, body } = media;
    if !ret.push(at_media.location(), TokenType::Keyword, 0) {
        return false;
    }
    fn find_in_media_query_list(ret: &mut SemanticTokenRet, list: &MediaQueryList) -> bool {
        match list {
            MediaQueryList::Unknown(x) => find_in_token_tree_list(ret, x),
            MediaQueryList::Sub(x) => find_in_children(ret, TokenType::Operator, x, |ret, x| {
                find_in_media_query_list(ret, x)
            }),
            MediaQueryList::And(x) => {
                x.iter().all(|(x, kw)| {
                    if !find_in_media_query_list(ret, x) { return false; }
                    match kw {
                        MediaAndKeyword::None => {}
                        MediaAndKeyword::And(x) => {
                            if !ret.push(x.location(), TokenType::Keyword, 0) {
                                return false;
                            }
                        }
                    }
                    true
                })
            }
            MediaQueryList::Or(x) => {
                x.iter().all(|(x, kw)| {
                    if !find_in_media_query_list(ret, x) { return false; }
                    match kw {
                        MediaOrKeyword::None => {}
                        MediaOrKeyword::Or(x) => {
                            if !ret.push(x.location(), TokenType::Keyword, 0) {
                                return false;
                            }
                        }
                        MediaOrKeyword::Comma(x) => {
                            if !ret.push(x.location(), TokenType::Operator, 0) {
                                return false;
                            }
                        }
                    }
                    true
                })
            }
            MediaQueryList::Not(kw, x) | MediaQueryList::Only(kw, x) => {
                ret.push(kw.location(), TokenType::Keyword, 0)
                    && find_in_media_query_list(ret, x)
            }
            MediaQueryList::MediaType(x) => {
                match x {
                    MediaType::Unknown(x) => ret.push(x.location(), TokenType::Type, 0),
                    MediaType::All(x)
                    | MediaType::Screen(x)
                    | MediaType::Print(x) => {
                        ret.push(x.location(), TokenType::Keyword, 0)
                    }
                }
            }
            MediaQueryList::MediaFeature(x) => {
                find_in_children(ret, TokenType::Operator, x, |ret, x| {
                    match x {
                        MediaFeature::Unknown(x) => find_in_token_tree_list(ret, x),
                        MediaFeature::Condition(x, colon, tt_list) => {
                            ret.push(x.location(), TokenType::Property, 0)
                                && ret.push(colon.location(), TokenType::Operator, 0)
                                && find_in_token_tree_list(ret, tt_list)
                        }
                    }
                })
            }
        }
    }
    if let Some(list) = list {
        find_in_media_query_list(ret, list);
    }
    find_in_option_brace_or_semicolon(ret, body, |ret, rules| {
        for rule in rules.iter() {
            if !find_in_rule(ret, rule) {
                return false;
            }
        }
        true
    })
}

fn find_in_font_face_rule(ret: &mut SemanticTokenRet, font: &FontFaceRule) -> bool {
    let FontFaceRule { at_font_face, body } = font;
    ret.push(at_font_face.location(), TokenType::Keyword, 0)
        && find_in_option_brace_or_semicolon_property(ret, body)
}

fn find_in_keyframes_rule(ret: &mut SemanticTokenRet, keyframes: &KeyframesRule) -> bool {
    let KeyframesRule { at_keyframes, name, body } = keyframes;
    ret.push(at_keyframes.location(), TokenType::Keyword, 0)
        && find_in_maybe_unknown(ret, name, |ret, name| {
            ret.push(name.location(), TokenType::Type, 0)
        })
        && find_in_option_brace_or_semicolon(ret, body, |ret, list| {
            for keyframe in list.iter() {
                let r = match keyframe {
                    Keyframe::Named { progress, body } => {
                        find_in_maybe_unknown(ret, progress, |ret, x| {
                            ret.push(x.location(), TokenType::Keyword, 0)
                        })
                            && find_in_option_brace_or_semicolon_property(ret, body)
                    }
                    Keyframe::Percentage { progress, body } => {
                        find_in_maybe_unknown(ret, progress, |ret, x| {
                            ret.push(x.location(), TokenType::Number, 0)
                        })
                            && find_in_option_brace_or_semicolon_property(ret, body)
                    }
                    Keyframe::Unknown(x) => find_in_token_tree_list(ret, x),
                };
                if !r { return false }
            }
            true
        })
}

fn find_in_style_rule(ret: &mut SemanticTokenRet, style_rule: &StyleRule) -> bool {
    let StyleRule { selector, brace } = style_rule;
    for selector in selector.iter() {
        let ret = match selector {
            Selector::Unknown(x) => find_in_token_tree_list(ret, &x),
            Selector::Universal(x) => ret.push(x.location(), TokenType::Operator, 0),
            Selector::TagName(x) => ret.push(x.location(), TokenType::Type, 0),
            Selector::Id(x) => ret.push(x.location(), TokenType::Type, 0),
            Selector::Class(op, x) => {
                ret.push(op.location(), TokenType::Operator, 0)
                    && ret.push(x.location(), TokenType::Type, 0)
            }
            Selector::Attribute(x) => {
                find_in_children(ret, TokenType::Operator, x, |ret, children| {
                    find_in_token_tree_list(ret, children)
                })
            }
            Selector::NextSibling(x) => ret.push(x.location(), TokenType::Operator, 0),
            Selector::Child(x) => ret.push(x.location(), TokenType::Operator, 0),
            Selector::Column(x, y) => {
                ret.push(x.location(), TokenType::Operator, 0)
                    && ret.push(y.location(), TokenType::Operator, 0)
            }
            Selector::SubsequentSibling(x) => ret.push(x.location(), TokenType::Operator, 0),
            Selector::Namespace(x) => ret.push(x.location(), TokenType::Operator, 0),
            Selector::PseudoClass(x, name) => {
                ret.push(x.location(), TokenType::Operator, 0)
                    && find_in_name_or_function(ret, name)
            }
            Selector::PseudoElement(x, y, name) => {
                ret.push(x.location(), TokenType::Operator, 0)
                    && ret.push(y.location(), TokenType::Operator, 0)
                    && find_in_name_or_function(ret, name)
            }
        };
        if !ret { return false; }
    }
    find_in_option_brace_or_semicolon_property(ret, brace)
}

fn find_in_name_or_function(ret: &mut SemanticTokenRet, name: &IdentOrFunction) -> bool {
    match name {
        IdentOrFunction::Ident(x) => ret.push(x.location(), TokenType::Type, 0),
        IdentOrFunction::Function(x) => {
            find_in_children(ret, TokenType::Function, x, |ret, children| {
                find_in_token_tree_list(ret, &children)
            })
        }
    }
}

fn find_in_property(ret: &mut SemanticTokenRet, prop: &Property) -> bool {
    let Property { name, colon, value, semicolon } = prop;
    ret.push(name.location(), TokenType::Property, 0)
        && ret.push(colon.location(), TokenType::Operator, 0)
        && find_in_token_tree_list(ret, value)
        && find_in_option(ret, semicolon, |ret, x| {
            ret.push(x.location(), TokenType::Operator, 0)
        })
}

fn find_in_option<T>(
    ret: &mut SemanticTokenRet,
    x: &Option<T>,
    f: impl FnOnce(&mut SemanticTokenRet, &T) -> bool,
) -> bool {
    match x {
        None => true,
        Some(x) => f(ret, x),
    }
}

fn find_in_option_brace_or_semicolon<T>(
    ret: &mut SemanticTokenRet,
    x: &Option<BraceOrSemicolon<T>>,
    f: impl FnOnce(&mut SemanticTokenRet, &T) -> bool,
) -> bool {
    find_in_option(ret, x, |ret, x| {
        match x {
            BraceOrSemicolon::Semicolon(x) => ret.push(x.location(), TokenType::Operator, 0),
            BraceOrSemicolon::Brace(x) => find_in_children(ret, TokenType::Operator, x, f),
            BraceOrSemicolon::UnknownBrace(x) => find_in_children(ret, TokenType::Operator, x, |_, _| true),
        }
    })
}

fn find_in_option_brace_or_semicolon_property(
    ret: &mut SemanticTokenRet,
    x: &Option<BraceOrSemicolon<List<RuleOrProperty>>>,
) -> bool {
    find_in_option_brace_or_semicolon(ret, x, |ret, rules| {
        for rp in rules.iter() {
            let r = match rp {
                RuleOrProperty::Rule(x) => find_in_rule(ret, x),
                RuleOrProperty::Property(x) => find_in_property(ret, x),
            };
            if !r { return false }
        }
        true
    })
}

fn find_in_maybe_unknown<T>(
    ret: &mut SemanticTokenRet,
    x: &MaybeUnknown<T>,
    f: impl FnOnce(&mut SemanticTokenRet, &T) -> bool,
) -> bool {
    match x {
        MaybeUnknown::Unknown(x) => find_in_token_tree_list(ret, x),
        MaybeUnknown::Normal(x, y) => f(ret, x) && find_in_token_tree_list(ret, y),
    }
}

fn find_in_children<T>(
    ret: &mut SemanticTokenRet,
    ty: TokenType,
    x: &impl TokenGroupExt<T>,
    f: impl FnOnce(&mut SemanticTokenRet, &T) -> bool,
) -> bool {
    ret.push(x.left(), ty, 0)
        && f(ret, x.children())
        && find_in_token_tree_list(ret, x.trailing())
        && ret.push(x.right(), TokenType::Operator, 0)
}

fn find_in_token_tree_list(ret: &mut SemanticTokenRet, tt_list: &[TokenTree]) -> bool {
    for tt in tt_list.iter() {
        if !find_in_token_tree(ret, tt) {
            return false;
        }
    }
    true
}

fn find_in_token_tree(ret: &mut SemanticTokenRet, tt: &TokenTree) -> bool {
    match tt {
        TokenTree::Ident(x) => {
            ret.push(x.location(), TokenType::Type, 0)
        }
        TokenTree::AtKeyword(x) => {
            ret.push(x.location(), TokenType::Keyword, 0)
        }
        TokenTree::Hash(x) => {
            ret.push(x.location(), TokenType::Number, 0)
        }
        TokenTree::IDHash(x) => {
            ret.push(x.location(), TokenType::Type, 0)
        }
        TokenTree::QuotedString(x) => {
            ret.push(x.location(), TokenType::String, 0)
        }
        TokenTree::UnquotedUrl(x) => {
            ret.push(x.location(), TokenType::String, 0)
        }
        TokenTree::Number(x) => {
            ret.push(x.location(), TokenType::Number, 0)
        }
        TokenTree::Percentage(x) => {
            ret.push(x.location(), TokenType::Number, 0)
        }
        TokenTree::Dimension(x) => {
            ret.push(x.location(), TokenType::Number, 0)
        }
        TokenTree::Colon(x) => {
            ret.push(x.location(), TokenType::Operator, 0)
        }
        TokenTree::Semicolon(x) => {
            ret.push(x.location(), TokenType::Operator, 0)
        }
        TokenTree::Comma(x) => {
            ret.push(x.location(), TokenType::Operator, 0)
        }
        TokenTree::Operator(x) => {
            ret.push(x.location(), TokenType::Operator, 0)
        }
        TokenTree::Function(x) => {
            find_in_children(ret, TokenType::Function, x, |ret, children| {
                find_in_token_tree_list(ret, &children)
            })
        }
        TokenTree::Paren(x) => {
            find_in_children(ret, TokenType::Operator, x, |ret, children| {
                find_in_token_tree_list(ret, &children)
            })
        }
        TokenTree::Bracket(x) => {
            find_in_children(ret, TokenType::Operator, x, |ret, children| {
                find_in_token_tree_list(ret, &children)
            })
        }
        TokenTree::Brace(x) => {
            find_in_children(ret, TokenType::Operator, x, |ret, children| {
                find_in_token_tree_list(ret, &children)
            })
        }
        TokenTree::BadUrl(x) => {
            ret.push(x.location(), TokenType::String, 0)
        }
        TokenTree::BadString(x) => {
            ret.push(x.location(), TokenType::String, 0)
        }
        TokenTree::BadOperator(x) => {
            ret.push(x.location(), TokenType::Operator, 0)
        }
    }
}
