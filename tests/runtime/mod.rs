use compiler::llvm::LlvmModule;
use compiler::runtime::{BuildDiagnostics, BuildStage, BuildStatus, RuntimeStage};
use std::process::Command;

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
        ir: r#"; ModuleID = 'runtime_test'
target triple = "x86_64-unknown-linux-gnu"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"

@message = private unnamed_addr constant [23 x i8] c"Hello from GraduaLuau\0A\00", align 1

declare void @glua_print(i8*)

define i32 @main() {
entry:
    %message_ptr = getelementptr inbounds [23 x i8], [23 x i8]* @message, i64 0, i64 0
    call void @glua_print(i8* %message_ptr)
    ret i32 0
}
"#
        .to_string(),
    };

    let mut stage = RuntimeStage::new();
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join(format!(
        "test_gradualuau_{}.exe",
        std::process::id()
    ));

    let diagnostics = stage.link(&output_path, &llvm_module).expect("runtime link should succeed");

    assert!(!diagnostics.has_errors());
    assert!(diagnostics.stages.len() >= 4);
    assert!(diagnostics
        .object_files
        .iter()
        .any(|path| path.ends_with("program.o")));
    assert!(diagnostics
        .linked_libraries
        .iter()
        .any(|library| library == "gradualuau_runtime"));

    let exe_output = Command::new(&output_path)
        .output()
        .expect("generated executable should run");
    assert!(exe_output.status.success());
    assert!(
        String::from_utf8_lossy(&exe_output.stdout).contains("Hello from GraduaLuau"),
        "executable stdout was: {}",
        String::from_utf8_lossy(&exe_output.stdout)
    );

    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn test_build_status_variants() {
    assert_eq!(BuildStatus::Success, BuildStatus::Success);
    assert_eq!(BuildStatus::Warning, BuildStatus::Warning);
    assert_eq!(BuildStatus::Error, BuildStatus::Error);
}
