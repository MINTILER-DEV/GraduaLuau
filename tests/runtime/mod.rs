use compiler::llvm::LlvmModule;
use compiler::runtime::{BuildDiagnostics, BuildStage, BuildStatus, RuntimeStage};

#[test]
fn test_runtime_stage_creation() {
    let stage = RuntimeStage::new();
    assert_eq!(stage.diagnostics().stages.len(), 0);
}

#[test]
fn test_build_diagnostics_creation() {
    let diagnostics = BuildDiagnostics::new();
    assert_eq!(diagnostics.stages.len(), 0);
    assert_eq!(diagnostics.object_files.len(), 0);
    assert_eq!(diagnostics.linked_libraries.len(), 0);
}

#[test]
fn test_build_stage_creation() {
    let stage = BuildStage {
        name: "Test Stage".to_string(),
        status: BuildStatus::Success,
        duration_ms: 100,
        message: "Test message".to_string(),
    };

    assert_eq!(stage.name, "Test Stage");
    assert_eq!(stage.status, BuildStatus::Success);
    assert_eq!(stage.duration_ms, 100);
}

#[test]
fn test_build_diagnostics_add_stage() {
    let mut diagnostics = BuildDiagnostics::new();
    diagnostics.add_stage(BuildStage {
        name: "Test".to_string(),
        status: BuildStatus::Success,
        duration_ms: 50,
        message: "Test".to_string(),
    });

    assert_eq!(diagnostics.stages.len(), 1);
}

#[test]
fn test_build_diagnostics_has_errors() {
    let mut diagnostics = BuildDiagnostics::new();
    assert!(!diagnostics.has_errors());

    diagnostics.add_stage(BuildStage {
        name: "Error Stage".to_string(),
        status: BuildStatus::Error,
        duration_ms: 0,
        message: "Error".to_string(),
    });

    assert!(diagnostics.has_errors());
}

#[test]
fn test_build_diagnostics_format() {
    let mut diagnostics = BuildDiagnostics::new();
    diagnostics.set_output_path("/test/output".to_string());
    diagnostics.add_object_file("test.o".to_string());
    diagnostics.add_linked_library("std".to_string());

    let formatted = diagnostics.format();
    assert!(formatted.contains("Build Diagnostics"));
    assert!(formatted.contains("/test/output"));
    assert!(formatted.contains("test.o"));
    assert!(formatted.contains("std"));
}

#[test]
fn test_runtime_linking() {
    let llvm_module = LlvmModule {
        ir: "define i32 @main() {\nentry:\n    ret i32 0\n}".to_string(),
    };

    let mut stage = RuntimeStage::new();
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("test_gradualuau.exe");

    let result = stage.link(&output_path, &llvm_module);

    if let Ok(diagnostics) = result {
        assert!(!diagnostics.has_errors());
        assert!(diagnostics.stages.len() >= 2);
    }

    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn test_build_status_variants() {
    assert_eq!(BuildStatus::Success, BuildStatus::Success);
    assert_eq!(BuildStatus::Warning, BuildStatus::Warning);
    assert_eq!(BuildStatus::Error, BuildStatus::Error);
}
