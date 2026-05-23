mod json;
pub use json::*;

#[derive(Debug, Clone)]
pub struct ShhhFsEntry {
    pub name: String,
    pub contents: Vec<u8>,
}

pub trait ShhhFsProvider: Send + Sync {
    fn entries(&self) -> &[ShhhFsEntry];
}
