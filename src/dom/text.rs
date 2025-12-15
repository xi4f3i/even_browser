use std::ops::{Deref, DerefMut};

use crate::dom::character_data::CharacterData;

#[derive(Debug)]
pub(crate) struct Text {
    base: CharacterData,
}

impl Text {
    pub(crate) fn new(data: &str) -> Text {
        Text {
            base: CharacterData::new(data),
        }
    }
}

impl Deref for Text {
    type Target = CharacterData;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
