use crate::Au;
use crate::style::{ComputedStyle, Length};

pub struct FontMetrics {
    metrics: font_kit::metrics::Metrics,
    size: Length,
}

impl FontMetrics {
    fn to_length(&self, font_units: f32) -> Length {
        Length(Au::from_f32_px(font_units * self.size.to_f32_px() / self.metrics.units_per_em as f32))
    }

    pub fn ascent(&self) -> Length {
        self.to_length(self.metrics.ascent)
    }

    pub fn descent(&self) -> Length {
        self.to_length(self.metrics.descent)
    }

    pub fn x_height(&self) -> Length {
        self.to_length(self.metrics.x_height)
    }

    pub fn cap_height(&self) -> Length {
        self.to_length(self.metrics.cap_height)
    }

    pub fn underline_position(&self) -> Length {
        self.to_length(self.metrics.underline_position)
    }

    pub fn underline_thickness(&self) -> Length {
        self.to_length(self.metrics.underline_position)
    }
}

impl FontMetrics {
    pub fn from_style(style: &ComputedStyle) -> Self {
        let mut loader = super::loader::Loader::new(style);
        let font = loader.first_available_font();
        trace!("FontMetrics::from_style() -> {}", font.full_name());
        FontMetrics {
            metrics: font.metrics(),
            size: style.font_size,
        }
    }
}

/* Useful test to dump some font metrics.
#[test]
fn test_font() {
    let mut style = ComputedStyle::initial();
    style.set_named_font_family("Bitstream Vera Sans");
    let style = style.finish(true);

    let metrics = FontMetrics::from_style(&style);
    panic!(
        "ascent = {}, descent = {}, x-height = {}, cap-height = {}, \
         underline-position = {}, underline-thickness = {}",
        metrics.ascent(), metrics.descent(), metrics.x_height(),
        metrics.cap_height(), metrics.underline_position(),
        metrics.underline_thickness(),
    );
}
*/
