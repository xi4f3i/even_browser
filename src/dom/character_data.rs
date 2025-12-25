#[derive(Debug)]
pub(crate) struct CharacterData {
    data: String,
}

impl CharacterData {
    pub(crate) fn new(data: &str) -> CharacterData {
        CharacterData {
            data: data.to_string(),
        }
    }

    pub(crate) fn data(&self) -> &str {
        &self.data
    }

    pub(crate) fn set_data(&mut self, data: String) {
        self.data = data;
    }

    pub(crate) fn length(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn push(&mut self, c: char) {
        self.data.push(c);
    }
}
