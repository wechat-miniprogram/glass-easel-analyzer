use crate::{utils::{exclusive_contains, exclusive_ordering, inclusive_contains}, wxss::{keyframe::Keyframe, media::*, rule::{IdentOrFunction, Selector}, token::*, CSSParse, List, MaybeUnknown, Position, Rule, RuleOrProperty, StyleSheet}};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum Token<'a> {
    None,
    Keyword(&'a Ident),
    Ident(&'a Ident),
    AtKeyword(&'a AtKeyword),
    Hash(&'a Hash),
    IDHash(&'a IDHash),
    QuotedString(&'a QuotedString),
    UnquotedUrl(&'a UnquotedUrl),
    Function(&'a Function<Vec<TokenTree>>),
    Paren(&'a Paren<Vec<TokenTree>>),
    Bracket(&'a Bracket<Vec<TokenTree>>),
    Brace(&'a Brace<Vec<TokenTree>>),
    BadUrl(&'a BadUrl),
    BadString(&'a BadString),
    TagName(&'a Ident),
    Id(&'a IDHash),
    Class(&'a Operator, &'a Ident),
    PseudoClass(&'a Colon, &'a IdentOrFunction),
    PseudoElement(&'a Colon, &'a Colon, &'a IdentOrFunction),
    PropertyName(&'a Ident),
    StyleRuleUnknownIdent(&'a Ident),
    FontFacePropertyName(&'a Ident),
    MediaType(&'a Ident),
    MediaFeatureName(&'a Ident),
    MediaQueryUnknownParen(&'a Paren<()>),
    KeyframesName(&'a Ident),
    KeyframeProgressName(&'a Ident),
    KeyframeProgressPercentage(&'a Percentage),
    ImportUrl(&'a QuotedString),
}

pub(crate) fn find_token_in_position(sheet: &StyleSheet, pos: Position) -> Token {
    sheet.items.iter().find_map(|x| find_in_rule(x, pos)).unwrap_or(Token::None)
}

fn find_in_rule(rule: &Rule, pos: Position) -> Option<Token> {
    match rule {
        Rule::Unknown(tt_list) => {
            match find_in_token_tree_list(&tt_list, pos) {
                Some(Token::Ident(x)) if tt_list.first().location() == x.location() => {
                    Some(Token::StyleRuleUnknownIdent(x))
                }
                x => x,
            }
        }
        Rule::Style(style_rule) => {
            style_rule
                .selector
                .iter_values()
                .find_map(|selector_list| selector_list.iter().find_map(|x| find_in_selector(x, pos)))
                .or_else(|| find_in_option_brace_or_semicolon_properties(&style_rule.brace, pos))
        }
        Rule::Import(import_rule) => {
            if inclusive_contains(&import_rule.at_import.location, pos) {
                Some(Token::AtKeyword(&import_rule.at_import))
            } else {
                find_in_maybe_unknown(&import_rule.url, pos, |t| {
                    inclusive_contains(&t.location, pos).then_some(Token::ImportUrl(t))
                }).or_else(|| find_in_token_tree_list(&import_rule.condition, pos))
            }
        }
        Rule::Media(media_rule) => {
            if inclusive_contains(&media_rule.at_media.location, pos) {
                Some(Token::AtKeyword(&media_rule.at_media))
            } else {
                media_rule
                    .list
                    .as_ref()
                    .and_then(|x| find_in_media_query_list(x, pos))
                    .or_else(|| {
                        find_in_option_brace_or_semicolon(&media_rule.body, pos, |x| {
                            x.iter().find_map(|x| find_in_rule(x, pos))
                        })
                    })
            }
        }
        Rule::FontFace(font_face_rule) => {
            if inclusive_contains(&font_face_rule.at_font_face.location, pos) {
                Some(Token::AtKeyword(&font_face_rule.at_font_face))
            } else {
                find_in_option_brace_or_semicolon_properties(&font_face_rule.body, pos)
                    .map(|x| {
                        match x {
                            Token::PropertyName(x) => Token::FontFacePropertyName(x),
                            x => x,
                        }
                    })
            }
        }
        Rule::Keyframes(keyframes_rule) => {
            if inclusive_contains(&keyframes_rule.at_keyframes.location, pos) {
                Some(Token::AtKeyword(&keyframes_rule.at_keyframes))
            } else {
                find_in_maybe_unknown(&keyframes_rule.name, pos, |t| {
                    inclusive_contains(&t.location, pos).then_some(Token::KeyframesName(t))
                }).or_else(|| {
                    find_in_option_brace_or_semicolon(&keyframes_rule.body, pos, |x| {
                        x.iter().find_map(|x| {
                            match x {
                                Keyframe::Named { progress, body } => {
                                    find_in_maybe_unknown(progress, pos, |t| {
                                        inclusive_contains(&t.location, pos).then_some(Token::KeyframeProgressName(t))
                                    }).or_else(|| {
                                        find_in_option_brace_or_semicolon_properties(body, pos)
                                    })
                                }
                                Keyframe::Percentage { progress, body } => {
                                    find_in_maybe_unknown(progress, pos, |t| {
                                        inclusive_contains(&t.location, pos).then_some(Token::KeyframeProgressPercentage(t))
                                    }).or_else(|| {
                                        find_in_option_brace_or_semicolon_properties(body, pos)
                                    })
                                }
                                Keyframe::Unknown(x) => {
                                    find_in_token_tree_list(x, pos)
                                }
                            }
                        })
                    })
                })
            }
        }
        Rule::UnknownAtRule(kw, x) => {
            if inclusive_contains(&kw.location, pos) {
                Some(Token::AtKeyword(&kw))
            } else {
                find_in_token_tree_list(&x, pos)
            }
        }
    }
}

fn find_in_media_query_list(x: &MediaQueryList, pos: Position) -> Option<Token> {
    if !inclusive_contains(&x.location(), pos) {
        return None;
    }
    match x {
        MediaQueryList::Unknown(x) => find_in_token_tree_list(x, pos),
        MediaQueryList::EmptyParen(x) => {
            if exclusive_contains(&x.location(), pos) {
                Some(Token::MediaQueryUnknownParen(x))
            } else {
                None
            }
        },
        MediaQueryList::Sub(x) => {
            find_in_children(x, pos, |x| {
                find_in_media_query_list(x, pos)
            })
        }
        MediaQueryList::Not(kw, x) | MediaQueryList::Only(kw, x) => {
            if inclusive_contains(&kw.location, pos) {
                Some(Token::Keyword(kw))
            } else {
                find_in_media_query_list(&x, pos)
            }
        }
        MediaQueryList::And(list) => {
            list.iter().find_map(|(cond, kw)| {
                find_in_media_query_list(cond, pos)
                    .or_else(|| {
                        match kw {
                            MediaAndKeyword::None => None,
                            MediaAndKeyword::And(kw) => {
                                inclusive_contains(&kw.location(), pos).then_some(Token::Keyword(kw))
                            }
                        }
                    })
            })
        }
        MediaQueryList::Or(list) => {
            list.iter().find_map(|(cond, kw)| {
                find_in_media_query_list(cond, pos)
                    .or_else(|| {
                        match kw {
                            MediaOrKeyword::None => None,
                            MediaOrKeyword::Comma(_) => None,
                            MediaOrKeyword::Or(kw) => {
                                inclusive_contains(&kw.location(), pos).then_some(Token::Keyword(kw))
                            }
                        }
                    })
            })
        }
        MediaQueryList::MediaType(x) => {
            if inclusive_contains(&x.location(), pos) {
                let kw = match x {
                    MediaType::Unknown(x) => x,
                    MediaType::All(x) => x,
                    MediaType::Screen(x) => x,
                    MediaType::Print(x) => x,
                };
                Some(Token::MediaType(kw))
            } else {
                None
            }
        }
        MediaQueryList::MediaFeature(x) => {
            find_in_children(x, pos, |x| {
                match x {
                    MediaFeature::Unknown(x) => find_in_token_tree_list(x, pos),
                    MediaFeature::SingleCondition(k) => {
                        Some(Token::MediaFeatureName(k))
                    }
                    MediaFeature::Condition(k, _, v) => {
                        if inclusive_contains(&k.location, pos) {
                            Some(Token::MediaFeatureName(k))
                        } else {
                            find_in_token_tree_list(&v, pos)
                        }
                    }
                }
            })
        }
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
        Selector::Class(op, x) => Token::Class(op, x),
        Selector::Attribute(x) => {
            find_in_children(x, pos, |x| {
                find_in_token_tree_list(x, pos)
            })?
        },
        Selector::PseudoClass(op, x) => Token::PseudoClass(op, x),
        Selector::PseudoElement(op1, op2, x) => Token::PseudoElement(op1, op2, x),
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
    x.iter().find_map(|x| {
        match x {
            RuleOrProperty::Rule(x) => find_in_rule(x, pos),
            RuleOrProperty::Property(x) => {
                if inclusive_contains(&x.name.location, pos) {
                    Some(Token::PropertyName(&x.name))
                } else {
                    find_in_token_tree_list(&x.value, pos)
                }
            }
        }
    })
}

fn find_in_option_brace_or_semicolon_properties(x: &Option<BraceOrSemicolon<List<RuleOrProperty>>>, pos: Position) -> Option<Token> {
    find_in_option_brace_or_semicolon(x, pos, |x| {
        find_in_rule_properties(x, pos)
    })
}

fn find_in_option_brace_or_semicolon<T>(
    x: &Option<BraceOrSemicolon<T>>,
    pos: Position,
    f: impl FnOnce(&T) -> Option<Token>,
) -> Option<Token> {
    match x.as_ref()? {
        BraceOrSemicolon::Brace(x) => {
            if !inclusive_contains(&x.location(), pos) {
                None
            } else {
                find_in_children(x, pos, f)
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

fn find_in_maybe_unknown<T>(x: &MaybeUnknown<T>, pos: Position, f: impl FnOnce(&T) -> Option<Token>) -> Option<Token> {
    match x {
        MaybeUnknown::Unknown(x) => find_in_token_tree_list(&x, pos),
        MaybeUnknown::Normal(x, tt_list) => {
            f(x).or_else(|| find_in_token_tree_list(tt_list, pos))
        }
    }
}

fn find_in_children<'a, T: 'a, P: TokenGroupExt<T>>(p: &'a P, pos: Position, f: impl FnOnce(&T) -> Option<Token>) -> Option<Token<'a>> {
    f(p.children()).or_else(|| find_in_token_tree_list(p.trailing(), pos))
}

fn find_in_token_tree_list(tt_list: &[TokenTree], pos: Position) -> Option<Token> {
    let index = tt_list.binary_search_by(|tt| {
        exclusive_ordering(&tt.location(), pos)
    });
    match index {
        Ok(index) => find_in_token_tree(&tt_list[index], pos),
        Err(index) => {
            let left = if index > 0 {
                find_in_token_tree(&tt_list[index - 1], pos)
            } else {
                None
            };
            let right = if index < tt_list.len() {
                find_in_token_tree(&tt_list[index], pos)
            } else {
                None
            };
            left.or(right).or_else(|| {
                if index > 0 && index < tt_list.len() {
                    Some(Token::None)
                } else {
                    None
                }
            })
        }
    }
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
            find_in_children(x, pos, |x| {
                find_in_token_tree_list(x, pos)
            }).or_else(|| {
                if !tt.location().contains(&pos) {
                    return None;
                }
                Some(Token::Function(x))
            })?
        }
        TokenTree::Paren(x) => {
            find_in_children(x, pos, |x| {
                find_in_token_tree_list(x, pos)
            }).or_else(|| {
                if !exclusive_contains(&tt.location(), pos) {
                    return None;
                }
                Some(Token::Paren(x))
            })?
        }
        TokenTree::Bracket(x) => {
            find_in_children(x, pos, |x| {
                find_in_token_tree_list(x, pos)
            }).or_else(|| {
                if !exclusive_contains(&tt.location(), pos) {
                    return None;
                }
                Some(Token::Bracket(x))
            })?
        }
        TokenTree::Brace(x) => {
            find_in_children(x, pos, |x| {
                find_in_token_tree_list(x, pos)
            }).or_else(|| {
                if !exclusive_contains(&tt.location(), pos) {
                    return None;
                }
                Some(Token::Brace(x))
            })?
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
