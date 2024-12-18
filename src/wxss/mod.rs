use std::ops::Range;

use glass_easel_template_compiler::parse::Position;

pub(crate) type Location = Range<Position>;

pub(crate) mod property;
pub(crate) mod rule;
pub(crate) mod token;

pub(crate) struct StyleSheet {
    pub(crate) items: Vec<Item>,
}

pub(crate) enum Item {
    Property(property::Property),
    UnknownExpression(),
    Style(rule::StyleRule),
    // Import(ImportRule),
    // Media(MediaRule),
    // FontFace(FontFaceRule),
    // KeyFrames(KeyFramesRule),
    // UnknownRule(UnknownRule),
}
