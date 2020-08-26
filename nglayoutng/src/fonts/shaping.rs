use crate::style::ComputedStyle;
use smallvec::SmallVec;
use euclid::default::Point2D;
use app_units::Au;
// use harfbuzz as hb;
// use unicode_script::Script;

// TODO: we may want to have one of these per _character_, not per glyph, and
// use that for efficient lookup (tagging whether we're a ligature start or such
// as we go).
//
// TODO: We definitely want to optimize these for memory usage and common cases
// where offset is zero.
pub struct GlyphInfo {
    /// The actual glyph id of the font.
    pub glyph_id: u32,
    /// Relative position of the glyph.
    pub offset: Point2D<Au>,
    /// Horizontal advance.
    pub advance: Au,
    /// Offset into the text.
    pub byte_offset: usize,
}

#[derive(Default)]
struct ShapedTextRun {
    glyphs: SmallVec<[GlyphInfo; 32]>,
}

// Usually there's only one font, but there may be many in the case of font
// fallback.
#[derive(Default)]
pub struct ShapedText(SmallVec<[ShapedTextRun; 1]>);

impl ShapedText {
    pub fn glyphs(&self) -> impl Iterator<Item = &GlyphInfo> {
        self.0.iter().flat_map(|run| run.glyphs.as_slice())
    }
}

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
