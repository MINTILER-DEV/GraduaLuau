use compiler::context::{CompilerContext, CompilerOptions, EmitOptions};
use compiler::pipeline;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn build_emits_requested_artifacts() {
    let source_path = temp_source(
        "build_emits_requested_artifacts",
        "function main()\nend\n",
    );
    let output_path = temp_output_path("build_emits_requested_artifacts");

    let mut options = CompilerOptions::default();
    options.output_path = output_path.clone();
    options.emit = EmitOptions {
        llvm: true,
        hir: true,
        mir: true,
        ast: true,
        tokens: true,
    };

    let mut context = CompilerContext::new(options);
    pipeline::build(&mut context, &source_path).expect("build should succeed");

    assert!(output_path.exists());
    assert_artifact(&output_path.with_extension("tokens"), "Function");
    assert_artifact(&output_path.with_extension("ast"), "Program");
    assert_artifact(&output_path.with_extension("hir"), "Module");
    assert_artifact(&output_path.with_extension("mir"), "Function");
    assert_artifact(&output_path.with_extension("ll"), "target triple");

    cleanup_artifacts(&output_path);
    let _ = fs::remove_file(&source_path);
}

#[test]
fn check_emits_requested_artifacts_without_executable() {
    let source_path = temp_source(
        "check_emits_requested_artifacts_without_executable",
        "function main()\nend\n",
    );
    let output_path = temp_output_path("check_emits_requested_artifacts_without_executable");

    let mut options = CompilerOptions::default();
    options.output_path = output_path.clone();
    options.emit = EmitOptions {
        llvm: true,
        hir: true,
        mir: true,
        ast: true,
        tokens: true,
    };

    let mut context = CompilerContext::new(options);
    pipeline::check(&mut context, &source_path).expect("check should succeed");

    assert!(!output_path.exists());
    assert_artifact(&output_path.with_extension("tokens"), "Function");
    assert_artifact(&output_path.with_extension("ast"), "Program");
    assert_artifact(&output_path.with_extension("hir"), "Module");
    assert_artifact(&output_path.with_extension("mir"), "Function");
    assert_artifact(&output_path.with_extension("ll"), "target triple");

    cleanup_artifacts(&output_path);
    let _ = fs::remove_file(&source_path);
}

fn temp_source(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    path.push(format!("gradualuau_{name}_{nanos}.glu"));
    fs::write(&path, contents).expect("source file should be writable");
    path
}

fn temp_output_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    path.push(format!("gradualuau_{name}_{nanos}.exe"));
    path
}

fn assert_artifact(path: &PathBuf, expected_fragment: &str) {
    let contents = fs::read_to_string(path).expect("artifact should exist");
    assert!(
        contents.contains(expected_fragment),
        "artifact {} did not contain expected fragment '{}'",
        path.display(),
        expected_fragment
    );
}

fn cleanup_artifacts(output_path: &PathBuf) {
    let _ = fs::remove_file(output_path);
    let _ = fs::remove_file(output_path.with_extension("tokens"));
    let _ = fs::remove_file(output_path.with_extension("ast"));
    let _ = fs::remove_file(output_path.with_extension("hir"));
    let _ = fs::remove_file(output_path.with_extension("mir"));
    let _ = fs::remove_file(output_path.with_extension("ll"));
}
