use std::ops::Range;

use glass_easel_template_compiler::parse::Position;

pub(crate) type Location = Range<Position>;

pub(crate) mod font_face;
pub(crate) mod import;
pub(crate) mod key_frame;
pub(crate) mod media;
pub(crate) mod property;
pub(crate) mod rule;
pub(crate) mod token;

pub(crate) struct StyleSheet {
    pub(crate) items: Vec<Item>,
}

pub(crate) enum Item {
    Unknown(Vec<token::TokenTree>),
    Property(property::Property),
    Style(rule::StyleRule),
    Import(import::ImportRule),
    Media(media::MediaRule),
    FontFace(font_face::FontFaceRule),
    KeyFrames(key_frame::KeyFramesRule),
    UnknownAtRule(token::AtKeyword, Vec<token::TokenTree>),
}
