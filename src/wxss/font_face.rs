use super::*;

#[derive(Debug, Clone)]
pub(crate) struct FontFaceRule {
    pub(crate) at_font_face: AtKeyword,
    pub(crate) body: Option<BraceOrSemicolon<List<RuleOrProperty>>>,
}

impl CSSParse for FontFaceRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_font_face = CSSParse::css_parse(ps)?;
        let body = CSSParse::css_parse(ps);
        Some(Self { at_font_face, body })
    }

    fn location(&self) -> Location {
        let start = self.at_font_face.location.start;
        let end = match self.body.as_ref() {
            None => self.at_font_face.location.end,
            Some(x) => x.location().end,
        };
        start..end
    }
}
