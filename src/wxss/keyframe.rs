use super::*;

pub(crate) struct KeyframesRule {
    at_keyframes: AtKeyword,
    name: Ident,
    body: Brace<Vec<Keyframe>>,
}

pub(crate) enum Keyframe {
    Named {
        progress: Ident,
        body: Brace<Vec<RuleOrProperty>>,
    },
    Percentage {
        progress: Percentage,
        body: Brace<Vec<RuleOrProperty>>,
    },
    Unknown(Rule),
}

impl CSSParse for KeyframesRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!() // TODO
    }
}
