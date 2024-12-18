use super::{token::{AtKeyword, Brace, Ident, Percentage, TokenTree}, Item};

pub(crate) struct KeyFramesRule {
    at_keyframes: AtKeyword,
    name: Ident,
    body: Brace<Vec<KeyFrame>>,
}

pub(crate) enum KeyFrame {
    Named {
        progress: Ident,
        body: Brace<Vec<Item>>,
    },
    Percentage {
        progress: Percentage,
        body: Brace<Vec<Item>>,
    },
    Unknown(Item),
}
