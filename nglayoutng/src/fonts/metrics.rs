use crate::Au;
use crate::style::{ComputedStyle, Length, SingleFontFamily, GenericFamily, FontStyle, FontWeight};
use font_kit::{
    loaders::freetype::Font,
    source::SystemSource,
};

fn best_font(style: &ComputedStyle) -> Font {
    use font_kit::family_name::FamilyName;
    use font_kit::properties::{Properties, Weight, Style};

    fn to_font_kit_family(f: &SingleFontFamily) -> FamilyName {
        match *f {
            SingleFontFamily::Generic(ref g) => match *g {
                GenericFamily::Serif => FamilyName::Serif,
                GenericFamily::SansSerif => FamilyName::SansSerif,
                GenericFamily::Monospace => FamilyName::Monospace,
            },
            SingleFontFamily::Named(ref named) => {
                FamilyName::Title(named.name.clone())
            },
        }
    }

    let mut font_kit_families =
        style.font_family.iter().map(to_font_kit_family).collect::<Vec<_>>();

    // Always append a last resort font.
    font_kit_families.push(FamilyName::Serif);

    let mut properties = Properties::new();
    properties
        .style(match style.font_style {
            FontStyle::Normal => Style::Normal,
            FontStyle::Italic => Style::Italic,
        })
        .weight(match style.font_weight {
            FontWeight::Normal => Weight::NORMAL,
            FontWeight::Bold => Weight::BOLD,
        });

    SystemSource::new().select_best_match(&font_kit_families, &properties).unwrap().load().unwrap()
}

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
        let font = best_font(style);
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
