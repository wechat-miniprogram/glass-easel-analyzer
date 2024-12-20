use super::*;

pub(crate) struct StyleRule {
    pub(crate) selector: Vec<Selector>,
    pub(crate) brace: Brace<Vec<Rule>>,
}

pub(crate) struct Selector {
    pub(crate) segments: Vec<SelectorSegment>,
    pub(crate) comma: Option<Comma>,
}

pub(crate) enum SelectorSegment {
    Universal(Operator),
    TagName(Ident),
    Id(IDHash),
    Class(Operator, Ident),
    Attribute(Bracket<TokenTree>),
    NextSibling(Operator),
    Child(Operator),
    Column(Operator),
    SubsequentSibling(Operator),
    Namespace(Operator),
    PseudoClass(Colon, IdentOrFunction),
    PseudoElement(Colon, Colon, IdentOrFunction),
}

pub(crate) enum IdentOrFunction {
    Ident(Ident),
    Function(Function<Vec<TokenTree>>),
}

impl CSSParse for StyleRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!() // TODO
    }
}
