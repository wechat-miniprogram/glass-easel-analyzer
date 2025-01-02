use super::*;

pub(crate) struct KeyframesRule {
    pub(crate) at_keyframes: AtKeyword,
    pub(crate) name: MaybeUnknown<Ident>,
    pub(crate) body: Option<BraceOrSemicolon<Vec<Keyframe>>>,
}

impl CSSParse for KeyframesRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_keyframes = CSSParse::css_parse(ps)?;
        let name = MaybeUnknown::parse_with_trailing(ps, |ps| ps.skip_until_before_brace_or_semicolon());
        let body = CSSParse::css_parse(ps);
        Some(Self { at_keyframes, name, body })
    }
}

pub(crate) enum Keyframe {
    Named {
        progress: MaybeUnknown<Ident>,
        body: Option<BraceOrSemicolon<Vec<RuleOrProperty>>>,
    },
    Percentage {
        progress: MaybeUnknown<Percentage>,
        body: Option<BraceOrSemicolon<Vec<RuleOrProperty>>>,
    },
    Unknown(Vec<TokenTree>),
}

impl CSSParse for Keyframe {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = match ps.peek()? {
            TokenTree::Ident(..) => {
                let progress = MaybeUnknown::parse_with_trailing(ps, |ps| ps.skip_until_before_brace_or_semicolon());
                let body = CSSParse::css_parse(ps);
                Self::Named { progress, body }
            }
            TokenTree::Percentage(..) => {
                let progress = MaybeUnknown::parse_with_trailing(ps, |ps| ps.skip_until_before_brace_or_semicolon());
                let body = CSSParse::css_parse(ps);
                Self::Percentage { progress, body }
            }
            _ => {
                Self::Unknown(ps.skip_until_before(|_| false))
            }
        };
        Some(ret)
    }
}
