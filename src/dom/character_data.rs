#[derive(Debug)]
pub(crate) struct CharacterData {
    data: String,
}

impl CharacterData {
    pub fn new(data: &str) -> CharacterData {
        CharacterData {
            data: data.to_string(),
        }
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn set_data(&mut self, data: String) {
        self.data = data;
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }
}
