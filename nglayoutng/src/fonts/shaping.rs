use crate::style::ComputedStyle;
use smallvec::SmallVec;
// use harfbuzz as hb;
// use unicode_script::Script;

#[derive(Default)]
struct ShapedTextRun {
    // TODO
}

// Usually there's only one font, but there may be many in the case of font
// fallback.
#[derive(Default)]
pub struct ShapedText(SmallVec<[ShapedTextRun; 1]>);

// TODO: Split if there's font fallback
pub fn shape(
    text: &str,
    style: &ComputedStyle,
) -> ShapedText {
    let mut loader = super::loader::Loader::new(style);

    // Itemize per font.
    let mut last_font = std::usize::MAX;
    let mut start = 0;
    let mut current = 0;
    let mut runs = SmallVec::<[_; 3]>::new();
    for c in text.chars() {
        let font = loader.font_for_character(c);
        if font != last_font && start != current {
            runs.push((start..current, font));
            start = current;
        }
        current += c.len_utf8();
        last_font = font;
    }

    if start != current {
        runs.push((start..current, last_font));
    }

    // TODO:
    // for &(range, font_index) in &runs {
    //     shape_run(..)
    // }

    Default::default()
}
