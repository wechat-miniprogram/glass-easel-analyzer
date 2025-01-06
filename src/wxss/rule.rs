use super::*;

#[derive(Debug, Clone)]
pub(crate) struct StyleRule {
    pub(crate) selector: Repeat<Selector, Comma>,
    pub(crate) brace: Option<BraceOrSemicolon<List<RuleOrProperty>>>,
}

impl CSSParse for StyleRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let selector = CSSParse::css_parse(ps)?;
        let brace = CSSParse::css_parse(ps);
        Some(Self { selector, brace })
    }

    fn location(&self) -> Location {
        let start = self.selector.location().start;
        let end = match self.brace.as_ref() {
            None => self.selector.location().end,
            Some(x) => x.location().end,
        };
        start..end
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Selector {
    Unknown(List<TokenTree>),
    Universal(Operator),
    TagName(Ident),
    Id(IDHash),
    Class(Operator, Ident),
    Attribute(Bracket<List<TokenTree>>),
    NextSibling(Operator),
    Child(Operator),
    Column(Operator, Operator),
    SubsequentSibling(Operator),
    Namespace(Operator),
    PseudoClass(Colon, IdentOrFunction),
    PseudoElement(Colon, Colon, IdentOrFunction),
}

impl CSSParse for Selector {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        fn collect_unknown(ps: &mut ParseState) -> Option<Selector> {
            let mut v = vec![];
            while let Some(peek) = ps.peek() {
                match peek {
                    TokenTree::Comma(_)
                    | TokenTree::Semicolon(_)
                    | TokenTree::Brace(_) => break,
                    _ => {
                        v.push(ps.next()?)
                    }
                }
            }
            if v.is_empty() { return None; }
            Some(Selector::Unknown(List::from_vec(v)))
        }
        let ret = match ps.peek()? {
            TokenTree::Semicolon(_)
            | TokenTree::Brace(_) => return None,
            TokenTree::Operator(x) => {
                if x.is("*") {
                    Self::Universal(CSSParse::css_parse(ps)?)
                } else if x.is(".") {
                    let op = CSSParse::css_parse(ps)?;
                    if let Some(TokenTree::Ident(..)) = ps.peek_with_whitespace() {
                        Self::Class(op, CSSParse::css_parse(ps)?)
                    } else {
                        Self::Unknown(List::from_vec(vec![TokenTree::Operator(op)]))
                    }
                } else if x.is("+") {
                    Self::NextSibling(CSSParse::css_parse(ps)?)
                } else if x.is(">") {
                    Self::Child(CSSParse::css_parse(ps)?)
                } else if x.is("~") {
                    Self::SubsequentSibling(CSSParse::css_parse(ps)?)
                } else if x.is("|") {
                    let op = CSSParse::css_parse(ps)?;
                    if let Some(TokenTree::Operator(x)) = ps.peek_with_whitespace() {
                        if x.is("|") {
                            let op2 = CSSParse::css_parse(ps)?;
                            Self::Column(op, op2)
                        } else {
                            Self::Namespace(CSSParse::css_parse(ps)?)
                        }
                    } else {
                        Self::Namespace(CSSParse::css_parse(ps)?)
                    }
                } else if x.is(":") {
                    let op = CSSParse::css_parse(ps)?;
                    if let Some(peek2) = ps.peek_with_whitespace() {
                        if peek2.is_ident_or_function() {
                            Self::PseudoClass(op, CSSParse::css_parse(ps)?)
                        } else if let TokenTree::Colon(_) = peek2 {
                            let op2 = CSSParse::css_parse(ps)?;
                            if ps.peek_with_whitespace().is_some_and(|x| x.is_ident_or_function()) {
                                Self::PseudoElement(op, op2, CSSParse::css_parse(ps)?)
                            } else {
                                Self::Unknown(List::from_vec(vec![TokenTree::Colon(op), TokenTree::Colon(op2)]))
                            }
                        } else {
                            Self::Unknown(List::from_vec(vec![TokenTree::Colon(op)]))
                        }
                    } else {
                        Self::Unknown(List::from_vec(vec![TokenTree::Colon(op)]))
                    }
                } else {
                    return collect_unknown(ps)
                }
            }
            TokenTree::Ident(_) => Self::TagName(CSSParse::css_parse(ps)?),
            TokenTree::IDHash(_) => Self::Id(CSSParse::css_parse(ps)?),
            TokenTree::Bracket(_) => Self::Attribute(CSSParse::css_parse(ps)?),
            _ => return collect_unknown(ps),
        };
        Some(ret)
    }

    fn location(&self) -> Location {
        match self {
            Self::Unknown(x) => {
                let first = x.first();
                let last = x.last();
                first.location().start..last.location().end
            }
            Self::Universal(x) => x.location(),
            Self::TagName(x) => x.location(),
            Self::Id(x) => x.location(),
            Self::Class(op, x) => {
                op.location().start..x.location().end
            }
            Self::Attribute(x) => x.location(),
            Self::NextSibling(x) => x.location(),
            Self::Child(x) => x.location(),
            Self::Column(x, y) => x.location().start..y.location().end,
            Self::SubsequentSibling(x) => x.location(),
            Self::Namespace(x) => x.location(),
            Self::PseudoClass(op, x) => {
                op.location().start..x.location().end
            }
            Self::PseudoElement(op, _, x) => {
                op.location().start..x.location().end
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum IdentOrFunction {
    Ident(Ident),
    Function(Function<Vec<TokenTree>>),
}

impl CSSParse for IdentOrFunction {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = match ps.peek()? {
            TokenTree::Ident(_) => Self::Ident(CSSParse::css_parse(ps)?),
            TokenTree::Function(_) => Self::Function(ps.parse_function(|ps| Some(ps.skip_to_end()))?),
            _ => return None,
        };
        Some(ret)
    }

    fn location(&self) -> Location {
        match self {
            Self::Ident(x) => x.location(),
            Self::Function(x) => x.location(),
        }
    }
}
