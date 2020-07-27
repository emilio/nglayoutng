use crate::style::ComputedStyle;
use smallvec::SmallVec;
// use harfbuzz as hb;
// use unicode_script::Script;

struct ShapedTextRun {
    // TODO
}

// Usually there's only one font, but there may be many in the case of font
// fallback.
pub struct ShapedText(SmallVec<[ShapedTextRun; 1]>);

// TODO: Split if there's font fallback
pub fn shape(
    _text: &str,
    _style: &ComputedStyle,
) -> ShapedText {
    unimplemented!()
}
