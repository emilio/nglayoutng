use crate::style::{ComputedStyle, SingleFontFamily, GenericFamily, FontStyle, FontWeight};
use font_kit::{
    family_name::FamilyName,
    loaders::freetype::Font,
    properties::Properties,
    source::SystemSource,
};

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

fn properties_for_style(style: &ComputedStyle) -> Properties {
    use font_kit::properties::{Weight, Style};
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
    properties
}

pub struct Loader {
    source: SystemSource,
    family_list: Vec<FamilyName>,
    properties: Properties,
    /// Lazily populated, the first available font as per
    /// https://drafts.csswg.org/css-fonts/#first-available-font
    first_available_font: Option<Font>,
    /// Lazily populated last-resort fallback font.
    system_fallback: Option<Font>,
    /// The set of cached fonts, one entry per entry in `family_list`, computed
    /// on demand.
    cached_fonts: Vec<Option<Font>>,
}

impl Loader {
    pub fn new(style: &ComputedStyle) -> Self {
        let source = SystemSource::new();
        let family_list =
            style.font_family.iter().map(to_font_kit_family).collect::<Vec<_>>();
        let properties = properties_for_style(style);
        Self {
            source,
            family_list,
            properties,
            first_available_font: None,
            system_fallback: None,
            cached_fonts: vec![],
        }
    }

    pub fn first_available_font(&mut self) -> &Font {
        if let Some(ref font) = self.first_available_font {
            return font;
        }
        if let Ok(handle) = self.source.select_best_match(&self.family_list, &self.properties) {
            if let Ok(font) = handle.load() {
                self.first_available_font = Some(font);
                return self.first_available_font.as_ref().unwrap()
            }
        }

        let fallback = self.fallback_font().clone();
        self.first_available_font = Some(fallback);
        self.first_available_font.as_ref().unwrap()
    }

    fn fallback_font(&mut self) -> &Font {
        if let Some(ref fallback) = self.system_fallback {
            return fallback;
        }
        let fallback = self.source.select_best_match(&[FamilyName::Serif], &self.properties).unwrap().load().unwrap();
        self.system_fallback = Some(fallback);
        self.system_fallback.as_ref().unwrap()
    }

    pub fn font_at(&mut self, i: usize) -> Option<&Font> {
        if i < self.cached_fonts.len() {
            return self.cached_fonts[i].as_ref();
        }
        if i >= self.family_list.len() {
            return Some(self.fallback_font());
        }

        let family = &self.family_list[i];
        debug_assert_eq!(self.cached_fonts.len(), i, "Should only query fonts in order");

        // TODO(emilio): Avoid clone? There seems to be no better API than
        // this...
        let family = self.source.select_best_match(
            &[family.clone()],
            &self.properties,
        ).ok().and_then(|f| f.load().ok());
        self.cached_fonts.push(family);
        self.cached_fonts[i].as_ref()
    }

    // FIXME(emilio): This probably needs to have some more context (previous /
    // next character?) to handle stuff like combining marks and such.
    pub fn font_for_character(&mut self, ch: char) -> usize {
        for i in 0..self.family_list.len() {
            if let Some(f) = self.font_at(i) {
                if f.glyph_for_char(ch).is_some() {
                    return i;
                }
            }
        }

        self.family_list.len()
    }
}
