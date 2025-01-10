use super::*;

#[derive(Debug, Clone)]
pub(crate) struct MediaRule {
    pub(crate) at_media: AtKeyword,
    pub(crate) list: Option<MediaQueryList>,
    pub(crate) list_str: String,
    pub(crate) body: Option<BraceOrSemicolon<List<Rule>>>,
}

impl CSSParse for MediaRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        ps.skip_comments();
        let at_media = CSSParse::css_parse(ps)?;
        let list_pos_start = ps.byte_index();
        let list = CSSParse::css_parse(ps);
        let list_pos_end = ps.byte_index();
        let list_str = ps
            .source_slice(list_pos_start..list_pos_end)
            .trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let body = CSSParse::css_parse(ps);
        Some(Self {
            at_media,
            list,
            list_str,
            body,
        })
    }

    fn location(&self) -> Location {
        let start = self.at_media.location().start;
        let end = match self.body.as_ref() {
            None => match &self.list {
                None => self.at_media.location().end,
                Some(x) => x.location().end,
            },
            Some(x) => x.location().end,
        };
        start..end
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MediaQueryList {
    Unknown(List<TokenTree>),
    EmptyParen(Paren<()>),
    Sub(Paren<Box<MediaQueryList>>),
    And(List<(MediaQueryList, MediaAndKeyword)>),
    Or(List<(MediaQueryList, MediaOrKeyword)>),
    Not(Ident, Box<MediaQueryList>),
    Only(Ident, Box<MediaQueryList>),
    MediaType(MediaType),
    MediaFeature(Paren<MediaFeature>),
}

impl CSSParse for MediaQueryList {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = Self::parse_or(ps)?;
        if let Some(TokenTree::Brace(_)) = ps.peek() {
            return Some(ret);
        }
        let trailing = ps.skip_until_before_brace_or_semicolon();
        if let Self::Or(mut x) = ret {
            if trailing.len() > 0 {
                x.push((Self::Unknown(List::from_vec(trailing)), MediaOrKeyword::None));
            }
            Some(Self::Or(x))
        } else if trailing.len() > 0 {
            let list = vec![(ret, MediaOrKeyword::None), (Self::Unknown(List::from_vec(trailing)), MediaOrKeyword::None)];
            Some(Self::Or(List::from_vec(list)))
        } else {
            Some(ret)
        }
    }

    fn location(&self) -> Location {
        match self {
            Self::Unknown(x) => {
                x.first().location().start..x.last().location().end
            }
            Self::EmptyParen(x) => {
                x.location()
            }
            Self::Sub(x) => x.location(),
            Self::And(x) => {
                let start = x.first().0.location().start;
                let end = {
                    let last = x.last();
                    match &last.1 {
                        MediaAndKeyword::None => last.0.location().end,
                        MediaAndKeyword::And(x) => x.location().end,
                    }
                };
                start..end
            }
            Self::Or(x) => {
                let start = x.first().0.location().start;
                let end = {
                    let last = x.last();
                    match &last.1 {
                        MediaOrKeyword::None => last.0.location().end,
                        MediaOrKeyword::Or(x) => x.location().end,
                        MediaOrKeyword::Comma(x) => x.location().end,
                    }
                };
                start..end
            }
            Self::Not(kw, x) | Self::Only(kw, x) => {
                let start = kw.location().start;
                let end = x.location().end;
                start..end
            }
            Self::MediaType(x) => x.location(),
            Self::MediaFeature(x) => x.location(),
        }
    }
}

impl MediaQueryList {
    fn parse_paren(ps: &mut ParseState) -> Option<Self> {
        let sub = ps.parse_paren(|ps| {
            let Some(peek) = ps.peek() else {
                return None;
            };
            let ret = if peek.is_keyword("not") || peek.is_keyword("only") {
                CSSParse::css_parse(ps)?
            } else if let TokenTree::Paren(_) = peek {
                CSSParse::css_parse(ps)?
            } else {
                return None;
            };
            Some(Box::new(ret))
        });
        sub
            .map(|x| MediaQueryList::Sub(x))
            .or_else(|| {
                Some(MediaQueryList::MediaFeature(CSSParse::css_parse(ps)?))
            })
            .or_else(|| {
                let empty = ps.parse_paren(|ps| {
                    if ps.peek().is_none() {
                        return Some(());
                    }
                    None
                });
                empty.map(|x| MediaQueryList::EmptyParen(x))
            })
    }

    fn parse_item(ps: &mut ParseState) -> Option<Self> {
        let peek = ps.peek()?;
        if peek.is_keyword("not") {
            let kw = CSSParse::css_parse(ps)?;
            Self::parse_item(ps).map(|x| Self::Not(kw, Box::new(x)))
        } else if peek.is_keyword("only") {
            let kw = CSSParse::css_parse(ps)?;
            Self::parse_item(ps).map(|x| Self::Only(kw, Box::new(x)))
        } else if peek.is_ident() {
            MediaType::css_parse(ps).map(|x| Self::MediaType(x))
        } else if let TokenTree::Paren(_) = peek {
            Self::parse_paren(ps)
        } else {
            None
        }
    }

    fn parse_and(ps: &mut ParseState) -> Option<Self> {
        let mut ret = vec![];
        let mut next = Some(Self::parse_item(ps)?);
        while ps.peek()?.is_keyword("and") {
            let and = match Ident::css_parse(ps) {
                None => MediaAndKeyword::None,
                Some(x) => MediaAndKeyword::And(x),
            };
            let n = Self::parse_item(ps);
            let prev = std::mem::replace(&mut next, n).unwrap();
            ret.push((prev, and));
            if next.is_none() {
                break;
            }
        }
        if ret.len() == 0 {
            Some(next.unwrap())
        } else {
            if let Some(next) = next {
                ret.push((next, MediaAndKeyword::None));
            }
            Some(Self::And(List::from_vec(ret)))
        }
    }

    fn parse_or(ps: &mut ParseState) -> Option<Self> {
        let mut ret = vec![];
        let mut next = Some(Self::parse_and(ps)?);
        while let Some(peek) = ps.peek() {
            let or = if let TokenTree::Comma(_) = peek {
                MediaOrKeyword::Comma(CSSParse::css_parse(ps)?)
            } else if peek.is_keyword("or") {
                MediaOrKeyword::Or(CSSParse::css_parse(ps)?)
            } else {
                break
            };
            let n = Self::parse_and(ps);
            let prev = std::mem::replace(&mut next, n).unwrap();
            ret.push((prev, or));
            if next.is_none() {
                break;
            }
        }
        if ret.len() == 0 {
            Some(next.unwrap())
        } else {
            if let Some(next) = next {
                ret.push((next, MediaOrKeyword::None));
            }
            Some(Self::Or(List::from_vec(ret)))
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MediaAndKeyword {
    None,
    And(Ident),
}

#[derive(Debug, Clone)]
pub(crate) enum MediaOrKeyword {
    None,
    Or(Ident),
    Comma(Comma),
}

#[derive(Debug, Clone)]
pub(crate) enum MediaType {
    Unknown(Ident),
    All(Ident),
    Screen(Ident),
    Print(Ident),
}

impl CSSParse for MediaType {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ident = Ident::css_parse(ps)?;
        let ret = match ident.content.as_str() {
            "all" => Self::All(ident),
            "screen" => Self::Screen(ident),
            "print" => Self::Print(ident),
            _ => Self::Unknown(ident),
        };
        Some(ret)
    }

    fn location(&self) -> Location {
        match self {
            Self::Unknown(x) => x.location(),
            Self::All(x) => x.location(),
            Self::Screen(x) => x.location(),
            Self::Print(x) => x.location(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MediaFeature {
    Unknown(List<TokenTree>),
    SingleCondition(Ident),
    Condition(Ident, Colon, Vec<TokenTree>),
}

impl CSSParse for MediaFeature {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = if let (Some(TokenTree::Ident(peek1)), p2) = ps.peek2() {
            if let Some(TokenTree::Colon(_)) = p2 {
                Self::Condition(CSSParse::css_parse(ps)?, CSSParse::css_parse(ps)?, ps.skip_to_end())
            } else if p2.is_none() {
                Self::SingleCondition(peek1)
            } else {
                Self::Unknown(CSSParse::css_parse(ps)?)
            }
        } else {
            Self::Unknown(CSSParse::css_parse(ps)?)
        };
        Some(ret)
    }

    fn location(&self) -> Location {
        match self {
            Self::Unknown(x) => {
                x.first().location().start..x.last().location().end
            }
            Self::SingleCondition(k) => {
                k.location()
            }
            Self::Condition(k, colon, v) => {
                let start = k.location().start;
                let end = v.last().map(|x| x.location().end).unwrap_or(colon.location().end);
                start..end
            }
        }
    }
}
