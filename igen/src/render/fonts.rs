use fontdue::FontSettings;
use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Font {
    Dina,
    DinaBold,
    Wellfleet,
}

pub struct FontCollection {
    fonts: HashMap<Font, fontdue::Font>,
}

impl FontCollection {
    pub fn new() -> Self {
        let fonts: HashMap<Font, fontdue::Font> = HashMap::new();

        FontCollection { fonts }
    }

    const fn get_font_bytes(font: Font) -> &'static [u8] {
        match font {
            Font::Dina => include_bytes!("../../assets/Dina/DinaRemasterII-01.ttf") as &[u8],

            Font::DinaBold => {
                include_bytes!("../../assets/Dina/DinaRemasterII-Bold-02.ttf") as &[u8]
            }

            Font::Wellfleet => {
                include_bytes!("../../assets/Wellfleet/Wellfleet-Regular.ttf") as &[u8]
            }
        }
    }

    pub fn load_font(&mut self, font: Font) -> fontdue::Font {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.fonts.entry(font) {
            entry.insert(
                fontdue::Font::from_bytes(Self::get_font_bytes(font), FontSettings::default())
                    .unwrap_or_else(|_| panic!("Could not load font: {:?}", font)),
            );
            // Cloning here is unnecessary, but it prevents some borrow checker errors later on
            // also: pff
            self.fonts.get(&font).unwrap().clone()
        } else {
            self.fonts.get(&font).unwrap().clone()
        }
    }
}
