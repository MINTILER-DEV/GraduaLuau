use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
    // map normalized paths to file ids to prevent duplicate loads
    path_map: HashMap<PathBuf, FileId>,
    // module request cache keyed by normalized module path
    module_cache: HashMap<PathBuf, FileId>,
    // stack of paths currently being resolved (for cycle detection)
    resolving: Vec<PathBuf>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            path_map: HashMap::new(),
            module_cache: HashMap::new(),
            resolving: Vec::new(),
        }
    }

    /// Load a file from disk. This will normalize the path, prevent
    /// duplicate loads, and ensure the `.glu` extension when missing.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<FileId, SourceError> {
        let mut candidate = path.as_ref().to_path_buf();

        // If the provided path has no extension, assume `.glu`.
        if candidate.extension().is_none() {
            candidate.set_extension("glu");
        }

        let norm = normalize_path(&candidate);

        if let Some(&id) = self.path_map.get(&norm) {
            return Ok(id);
        }

        let text = fs::read_to_string(&candidate).map_err(|source| SourceError::Read {
            path: candidate.clone(),
            source,
        })?;

        Ok(self.add_file(candidate, text))
    }

    /// Add a file into the manager. If the normalized path already exists,
    /// the existing `FileId` is returned.
    pub fn add_file(&mut self, path: PathBuf, text: String) -> FileId {
        let norm = normalize_path(&path);

        if let Some(&id) = self.path_map.get(&norm) {
            return id;
        }

        let id = FileId(self.files.len());
        self.files.push(SourceFile::new(id, norm.clone(), text));
        self.path_map.insert(norm, id);
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    pub fn get_by_path(&self, path: impl AsRef<Path>) -> Option<&SourceFile> {
        let norm = normalize_path(path.as_ref());
        self.path_map.get(&norm).and_then(|id| self.get(*id))
    }

    /// Resolve a module request from a `current` file. Only relative
    /// resolution is supported (requests starting with `.`).
    pub fn resolve_module(&mut self, current: FileId, module: &str) -> Result<FileId, SourceError> {
        if !module.starts_with('.') {
            return Err(SourceError::UnsupportedModule(module.to_string()));
        }

        let current_file = self.get(current).ok_or_else(|| SourceError::NotFound(PathBuf::new()))?;
        let parent = current_file.path.parent().unwrap_or(Path::new(""));

        let mut candidate = parent.join(module);
        if candidate.extension().is_none() {
            candidate.set_extension("glu");
        }

        let norm = normalize_path(&candidate);

        if let Some(&id) = self.module_cache.get(&norm) {
            return Ok(id);
        }

        // Cycle detection: if we're already resolving this path, report a cycle
        if self.resolving.iter().any(|p| p == &norm) {
            let mut chain = self.resolving.clone();
            chain.push(norm.clone());
            return Err(SourceError::Circular(chain));
        }

        if let Some(&id) = self.path_map.get(&norm) {
            self.module_cache.insert(norm.clone(), id);
            return Ok(id);
        }

        // attempt to read from disk
        self.resolving.push(norm.clone());
        let text = fs::read_to_string(&candidate).map_err(|source| {
            self.resolving.pop();
            SourceError::Read {
                path: candidate.clone(),
                source,
            }
        })?;

        let id = self.add_file(candidate, text);
        self.resolving.pop();
        self.module_cache.insert(norm.clone(), id);

        Ok(id)
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

impl FileId {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    file_id: FileId,
    start: usize,
    end: usize,
}

impl SourceSpan {
    pub fn new(file_id: FileId, start: usize, end: usize) -> Self {
        Self {
            file_id,
            start,
            end: end.max(start),
        }
    }

    pub fn file_id(self) -> FileId {
        self.file_id
    }

    pub fn start(self) -> usize {
        self.start
    }

    pub fn end(self) -> usize {
        self.end
    }

    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub file_id: FileId,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

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

    pub fn location(&self, offset: usize) -> Option<SourceLocation> {
        if offset > self.text.len() {
            return None;
        }

        let line_index = match self.line_offsets.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_offsets[line_index];

        Some(SourceLocation {
            file_id: self.id,
            offset,
            line: line_index + 1,
            column: offset.saturating_sub(line_start) + 1,
        })
    }

    pub fn line_text(&self, one_based_line: usize) -> Option<&str> {
        let line_index = one_based_line.checked_sub(1)?;
        let start = *self.line_offsets.get(line_index)?;
        let end = self.line_end(one_based_line)?;

        Some(self.text[start..end].trim_end_matches(['\r', '\n']))
    }

    pub fn line_end(&self, one_based_line: usize) -> Option<usize> {
        let line_index = one_based_line.checked_sub(1)?;
        let start = *self.line_offsets.get(line_index)?;
        let next_start = self.line_offsets.get(line_index + 1).copied();

        Some(match next_start {
            Some(next_start) => next_start,
            None => self.text.len(),
        }
        .max(start))
    }
}

#[derive(Debug)]
pub enum SourceError {
    Read { path: PathBuf, source: io::Error },
    NotFound(PathBuf),
    UnsupportedModule(String),
    Circular(Vec<PathBuf>),
}

impl Display for SourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "failed to read '{}': {source}", path.display())
            }
            Self::NotFound(path) => write!(formatter, "source file not found: {}", path.display()),
            Self::UnsupportedModule(m) => write!(formatter, "unsupported module request: {m}"),
            Self::Circular(chain) => {
                write!(formatter, "circular module resolution:")?;
                for p in chain {
                    write!(formatter, " {}", p.display())?;
                }
                Ok(())
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

/// Lexically normalize a path by collapsing `.` and `..` components.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for comp in path.components() {
        match comp {
            Component::Prefix(_) | Component::RootDir => components.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if components.is_empty() {
                    components.push(comp.as_os_str());
                } else {
                    // pop last non-root component if possible
                    components.pop();
                }
            }
            Component::Normal(c) => components.push(c),
        }
    }

    let mut out = PathBuf::new();
    for c in components {
        out.push(c);
    }

    out
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

    #[test]
    fn resolves_line_and_column_locations() {
        let mut manager = SourceManager::new();
        let id = manager.add_file(
            PathBuf::from("main.glu"),
            String::from("local x = 1\nprint(x)"),
        );
        let file = manager.get(id).expect("source file should exist");
        let location = file.location(12).expect("location should resolve");

        assert_eq!(location.line, 2);
        assert_eq!(location.column, 1);
        assert_eq!(file.line_text(2), Some("print(x)"));
    }

    #[test]
    fn prevents_duplicate_loads() {
        let mut manager = SourceManager::new();
        let id1 = manager.add_file(PathBuf::from("src/math.glu"), String::from("local a = 1"));
        let id2 = manager.add_file(PathBuf::from("src/./math.glu"), String::from("local a = 1"));

        assert_eq!(id1, id2);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn resolves_relative_module() {
        let mut manager = SourceManager::new();
        let main = manager.add_file(PathBuf::from("src/main.glu"), String::from("require('./math')"));
        let math = manager.add_file(PathBuf::from("src/math.glu"), String::from("local a = 1"));

        let resolved = manager.resolve_module(main, "./math").expect("should resolve module");
        assert_eq!(resolved, math);
    }
}
