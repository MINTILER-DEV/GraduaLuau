use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<FileId, SourceError> {
        let path = path.as_ref().to_path_buf();
        let text = fs::read_to_string(&path).map_err(|source| SourceError::Read {
            path: path.clone(),
            source,
        })?;

        Ok(self.add_file(path, text))
    }

    pub fn add_file(&mut self, path: PathBuf, text: String) -> FileId {
        let id = FileId(self.files.len());
        self.files.push(SourceFile::new(id, path, text));
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    id: FileId,
    path: PathBuf,
    text: String,
    line_offsets: Vec<usize>,
}

impl SourceFile {
    fn new(id: FileId, path: PathBuf, text: String) -> Self {
        let line_offsets = line_offsets(&text);

        Self {
            id,
            path,
            text,
            line_offsets,
        }
    }

    pub fn id(&self) -> FileId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    pub fn line_start(&self, zero_based_line: usize) -> Option<usize> {
        self.line_offsets.get(zero_based_line).copied()
    }
}

#[derive(Debug)]
pub enum SourceError {
    Read { path: PathBuf, source: io::Error },
}

impl Display for SourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "failed to read '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for SourceError {}

fn line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];

    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' && index + 1 < text.len() {
            offsets.push(index + 1);
        }
    }

    offsets
}

#[cfg(test)]
mod tests {
    use super::SourceManager;
    use std::path::PathBuf;

    #[test]
    fn tracks_line_offsets() {
        let mut manager = SourceManager::new();
        let id = manager.add_file(PathBuf::from("main.glu"), String::from("a\nb\nc"));
        let file = manager.get(id).expect("source file should exist");

        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_start(0), Some(0));
        assert_eq!(file.line_start(1), Some(2));
        assert_eq!(file.line_start(2), Some(4));
    }
}
