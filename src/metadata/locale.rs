#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetadataLocale {
    pub language: Option<String>,
    pub region: Option<String>,
}

impl MetadataLocale {
    pub fn is_empty(&self) -> bool {
        self.language.is_none() && self.region.is_none()
    }
}
