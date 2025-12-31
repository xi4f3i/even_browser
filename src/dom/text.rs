use crate::dom::character_data::CharacterData;
use crate::dom::html::node::TNodePtr;
use std::ops::{Deref, DerefMut};

/// https://dom.spec.whatwg.org/#interface-text
pub(crate) struct Text {
    base: CharacterData,
}

impl Text {
    pub(crate) fn new(parent: Option<TNodePtr>, data: &str) -> Text {
        Text {
            base: CharacterData::new(parent, data),
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
