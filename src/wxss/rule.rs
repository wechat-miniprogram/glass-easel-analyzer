use super::*;

pub(crate) struct StyleRule {
    pub(crate) selector: Repeat<Selector, Comma>,
    pub(crate) brace: Option<BraceOrSemicolon<Vec<Rule>>>,
}

impl CSSParse for StyleRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let selector = CSSParse::css_parse(ps)?;
        let brace = CSSParse::css_parse(ps);
        Some(Self { selector, brace })
    }
}

pub(crate) enum Selector {
    Unknown(Vec<TokenTree>),
    Universal(Operator),
    TagName(Ident),
    Id(IDHash),
    Class(Operator, Ident),
    Attribute(Bracket<TokenTree>),
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
            Some(Selector::Unknown(v))
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
                        Self::Unknown(vec![TokenTree::Operator(op)])
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
                                Self::Unknown(vec![TokenTree::Colon(op), TokenTree::Colon(op2)])
                            }
                        } else {
                            Self::Unknown(vec![TokenTree::Colon(op)])
                        }
                    } else {
                        Self::Unknown(vec![TokenTree::Colon(op)])
                    }
                } else if x.is("::") {
                    todo!()
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
}

pub(crate) enum IdentOrFunction {
    Ident(Ident),
    Function(Function<Vec<TokenTree>>),
}

impl CSSParse for IdentOrFunction {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = match ps.peek()? {
            TokenTree::Ident(_) => Self::Ident(CSSParse::css_parse(ps)?),
            TokenTree::Function(_) => Self::Function(CSSParse::css_parse(ps)?),
            _ => return None,
        };
        Some(ret)
    }
}
