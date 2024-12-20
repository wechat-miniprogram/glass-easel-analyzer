use super::*;

pub(crate) struct FontFaceRule {
    at_font_face: AtKeyword,
    body: Brace<Vec<RuleOrProperty>>,
}

impl CSSParse for FontFaceRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!() // TODO
    }
}
