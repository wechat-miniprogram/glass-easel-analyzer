use super::*;

#[derive(Debug, Clone)]
pub(crate) struct KeyframesRule {
    pub(crate) at_keyframes: AtKeyword,
    pub(crate) name: MaybeUnknown<Ident>,
    pub(crate) body: Option<BraceOrSemicolon<List<Keyframe>>>,
}

impl CSSParse for KeyframesRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_keyframes = CSSParse::css_parse(ps)?;
        let name = MaybeUnknown::parse_with_trailing(ps, |ps| ps.skip_until_before_brace_or_semicolon());
        let body = CSSParse::css_parse(ps);
        Some(Self { at_keyframes, name, body })
    }

    fn location(&self) -> Location {
        let start = self.at_keyframes.location().start;
        let end = match self.body.as_ref() {
            None => self.name.location().unwrap_or(self.at_keyframes.location()).end,
            Some(x) => x.location().end,
        };
        start..end
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Keyframe {
    Named {
        progress: MaybeUnknown<Ident>,
        body: Option<BraceOrSemicolon<List<RuleOrProperty>>>,
    },
    Percentage {
        progress: MaybeUnknown<Percentage>,
        body: Option<BraceOrSemicolon<List<RuleOrProperty>>>,
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

    fn location(&self) -> Location {
        match self {
            Self::Named { progress, body } => {
                let start = progress.location().unwrap().start;
                let end = match body.as_ref() {
                    None => progress.location().unwrap().end,
                    Some(x) => x.location().end,
                };
                start..end
            }
            Self::Percentage { progress, body } => {
                let start = progress.location().unwrap().start;
                let end = match body.as_ref() {
                    None => progress.location().unwrap().end,
                    Some(x) => x.location().end,
                };
                start..end
            }
            Self::Unknown(x) => {
                let start = x.first().unwrap().location().start;
                let end = x.last().unwrap().location().end;
                start..end
            }
        }
    }
}
