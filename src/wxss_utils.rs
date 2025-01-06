use std::ops::Range;

use crate::{utils::{exclusive_contains, inclusive_contains}, wxss::{rule::{IdentOrFunction, Selector}, token::*, CSSParse, List, Position, Rule, RuleOrProperty, StyleSheet}};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Token<'a> {
    None,
    Ident(&'a Ident),
    AtKeyword(&'a AtKeyword),
    Hash(&'a Hash),
    IDHash(&'a IDHash),
    QuotedString(&'a QuotedString),
    UnquotedUrl(&'a UnquotedUrl),
    BadUrl(&'a BadUrl),
    BadString(&'a BadString),
    TagName(&'a Ident),
    Id(&'a IDHash),
    Class(&'a Ident),
    PseudoClass(&'a IdentOrFunction),
    PseudoElement(&'a IdentOrFunction),
}

pub(crate) fn find_token_in_position(sheet: &StyleSheet, pos: Position) -> Option<Token> {
    sheet.items.iter().find_map(|x| find_in_rule(x, pos))
}

fn find_in_rule(rule: &Rule, pos: Position) -> Option<Token> {
    match rule {
        Rule::Style(style_rule) => {
            style_rule.selector.iter().find_map(|x| find_in_selector(x, pos))
                .or_else(|| find_in_option_brace_or_semicolon_properties(&style_rule.brace, pos))
        }
        _ => todo!() // TODO
    }
}

fn find_in_selector(selector: &Selector, pos: Position) -> Option<Token> {
    if !inclusive_contains(&selector.location(), pos) {
        return None;
    }
    let ret = match selector {
        Selector::Unknown(x) => find_in_token_tree_list(x, pos)?,
        Selector::TagName(x) => Token::TagName(x),
        Selector::Id(x) => Token::Id(x),
        Selector::Class(_, x) => Token::Class(x),
        Selector::Attribute(x) => find_in_token_tree_list(x.children(), pos)?,
        Selector::PseudoClass(_, x) => Token::PseudoClass(x),
        Selector::PseudoElement(_, _, x) => Token::PseudoElement(x),
        Selector::NextSibling(_)
        | Selector::Child(_)
        | Selector::Column(_, _)
        | Selector::SubsequentSibling(_)
        | Selector::Namespace(_)
        | Selector::Universal(_) => {
            if !exclusive_contains(&selector.location(), pos) {
                return None;
            }
            Token::None
        }
    };
    Some(ret)
}

fn find_in_rule_properties(x: &[RuleOrProperty], pos: Position) -> Option<Token> {
    todo!() // TODO
}

fn find_in_option_brace_or_semicolon_properties(x: &Option<BraceOrSemicolon<List<RuleOrProperty>>>, pos: Position) -> Option<Token> {
    match x.as_ref()? {
        BraceOrSemicolon::Brace(x) => {
            if !inclusive_contains(&x.location(), pos) {
                None
            } else {
                find_in_rule_properties(&x.children, pos)
                    .or_else(|| find_in_token_tree_list(&x.trailing, pos))
            }
        }
        BraceOrSemicolon::UnknownBrace(x) => {
            if !inclusive_contains(&x.location(), pos) {
                None
            } else {
                find_in_token_tree_list(&x.trailing, pos)
            }
        }
        BraceOrSemicolon::Semicolon(..) => {
            None
        }
    }
}

fn find_in_token_tree_list(tt_list: &[TokenTree], pos: Position) -> Option<Token> {
    tt_list.iter().find_map(|x| find_in_token_tree(x, pos))
}

fn find_in_token_tree(tt: &TokenTree, pos: Position) -> Option<Token> {
    if !inclusive_contains(&tt.location(), pos) {
        return None;
    }
    let ret = match tt {
        TokenTree::Ident(x) => Token::Ident(x),
        TokenTree::AtKeyword(x) => Token::AtKeyword(x),
        TokenTree::Hash(x) => Token::Hash(x),
        TokenTree::IDHash(x) => Token::IDHash(x),
        TokenTree::QuotedString(x) => Token::QuotedString(x),
        TokenTree::UnquotedUrl(x) => Token::UnquotedUrl(x),
        TokenTree::Function(x) => {
            find_in_token_tree_list(&x.children, pos)
                .or_else(|| find_in_token_tree_list(&x.trailing, pos))?
        }
        TokenTree::Paren(x) => {
            find_in_token_tree_list(&x.children, pos)
                .or_else(|| find_in_token_tree_list(&x.trailing, pos))?
        }
        TokenTree::Bracket(x) => {
            find_in_token_tree_list(&x.children, pos)
                .or_else(|| find_in_token_tree_list(&x.trailing, pos))?
        }
        TokenTree::Brace(x) => {
            find_in_token_tree_list(&x.children, pos)
                .or_else(|| find_in_token_tree_list(&x.trailing, pos))?
        }
        TokenTree::BadUrl(x) => Token::BadUrl(x),
        TokenTree::BadString(x) => Token::BadString(x),
        TokenTree::Number(_)
        | TokenTree::Percentage(_)
        | TokenTree::Dimension(_)
        | TokenTree::Colon(_)
        | TokenTree::Semicolon(_)
        | TokenTree::Comma(_)
        | TokenTree::Operator(_)
        | TokenTree::BadOperator(_) => {
            if !exclusive_contains(&tt.location(), pos) {
                return None;
            }
            Token::None
        }
    };
    Some(ret)
}
