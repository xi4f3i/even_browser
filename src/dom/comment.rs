use std::ops::{Deref, DerefMut};

use crate::dom::character_data::CharacterData;

#[derive(Debug)]
pub(crate) struct Comment {
    base: CharacterData,
}

impl Comment {
    pub(crate) fn new(data: &str) -> Comment {
        Comment {
            base: CharacterData::new(data),
        }
    }
}

impl Deref for Comment {
    type Target = CharacterData;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Comment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
