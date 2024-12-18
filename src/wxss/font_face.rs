use super::{token::{AtKeyword, Brace}, Item};

pub(crate) struct FontFaceRule {
    at_font_face: AtKeyword,
    body: Brace<Vec<Item>>,
}
