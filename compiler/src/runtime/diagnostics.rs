use std::fmt;

#[derive(Debug, Clone)]
pub struct BuildDiagnostics {
    pub stages: Vec<BuildStage>,
    pub duration_ms: u64,
    pub output_path: String,
    pub object_files: Vec<String>,
    pub linked_libraries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildStage {
    pub name: String,
    pub status: BuildStatus,
    pub duration_ms: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildStatus {
    Success,
    Warning,
    Error,
}

impl BuildDiagnostics {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            duration_ms: 0,
            output_path: String::new(),
            object_files: Vec::new(),
            linked_libraries: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: BuildStage) {
        self.stages.push(stage);
    }

    pub fn set_output_path(&mut self, path: String) {
        self.output_path = path;
    }

    pub fn add_object_file(&mut self, file: String) {
        self.object_files.push(file);
    }

    pub fn add_linked_library(&mut self, lib: String) {
        self.linked_libraries.push(lib);
    }

    pub fn has_errors(&self) -> bool {
        self.stages.iter().any(|stage| stage.status == BuildStatus::Error)
    }

    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str("Build Diagnostics:\n");
        output.push_str(&format!("  Duration: {}ms\n", self.duration_ms));
        output.push_str(&format!("  Output: {}\n", self.output_path));
        output.push_str(&format!("  Object Files: {}\n", self.object_files.len()));
        output.push_str(&format!("  Linked Libraries: {}\n", self.linked_libraries.len()));

        if !self.object_files.is_empty() {
            output.push_str("  Object File List:\n");
            for object_file in &self.object_files {
                output.push_str(&format!("    - {}\n", object_file));
            }
        }

        if !self.linked_libraries.is_empty() {
            output.push_str("  Linked Library List:\n");
            for library in &self.linked_libraries {
                output.push_str(&format!("    - {}\n", library));
            }
        }

        output.push_str("\nStages:\n");

        for stage in &self.stages {
            let status = match stage.status {
                BuildStatus::Success => "OK",
                BuildStatus::Warning => "WARN",
                BuildStatus::Error => "ERR",
            };

            output.push_str(&format!(
                "  {} {} ({}ms): {}\n",
                status, stage.name, stage.duration_ms, stage.message
            ));
        }

        output
    }
}

impl Default for BuildDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BuildDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}
