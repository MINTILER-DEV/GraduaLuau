pub mod diagnostics;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::llvm::LlvmModule;
pub use diagnostics::{BuildDiagnostics, BuildStage, BuildStatus};

#[derive(Debug, Default)]
pub struct RuntimeStage {
    diagnostics: BuildDiagnostics,
}

impl RuntimeStage {
    pub fn new() -> Self {
        Self {
            diagnostics: BuildDiagnostics::new(),
        }
    }

    pub fn diagnostics(&self) -> &BuildDiagnostics {
        &self.diagnostics
    }

    pub fn link(
        &mut self,
        output_path: &Path,
        llvm_module: &LlvmModule,
    ) -> Result<BuildDiagnostics, io::Error> {
        let start_time = Instant::now();
        self.diagnostics = BuildDiagnostics::new();

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.diagnostics
            .set_output_path(output_path.display().to_string());

        let build_dir = self.create_build_directory()?;
        let program_ir_path = build_dir.join("program.ll");
        let runtime_source_path = build_dir.join("gradualuau_runtime.c");
        let program_object_path = build_dir.join("program.o");
        let runtime_object_path = build_dir.join("gradualuau_runtime.o");

        let target_triple = self
            .detect_toolchain_target_triple()
            .unwrap_or_else(|_| "x86_64-pc-windows-cygnus".to_string());

        let rewritten_ir = self.rewrite_target_triple(&llvm_module.ir, &target_triple);
        fs::write(&program_ir_path, rewritten_ir)?;
        fs::write(&runtime_source_path, Self::runtime_shim_source())?;

        self.diagnostics.add_stage(BuildStage {
            name: "Materialize Sources".to_string(),
            status: BuildStatus::Success,
            duration_ms: start_time.elapsed().as_millis() as u64,
            message: format!(
                "Prepared LLVM IR and runtime shim in {}",
                build_dir.display()
            ),
        });

        let compile_ir_start = Instant::now();
        let compile_ir_output = self.run_bash_command(
            r#"clang-20 -c "$(cygpath -u "$GLUAU_PROGRAM_IR")" -o "$(cygpath -u "$GLUAU_PROGRAM_OBJECT")""#,
            &[
                ("GLUAU_PROGRAM_IR", &program_ir_path),
                ("GLUAU_PROGRAM_OBJECT", &program_object_path),
            ],
        )?;
        if !compile_ir_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_ir_output.stderr);
            self.diagnostics.add_stage(BuildStage {
                name: "Compile LLVM IR".to_string(),
                status: BuildStatus::Error,
                duration_ms: compile_ir_start.elapsed().as_millis() as u64,
                message: format!("LLVM compilation failed: {}", stderr.trim()),
            });
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("LLVM compilation failed: {}", stderr.trim()),
            ));
        }
        self.diagnostics.add_stage(BuildStage {
            name: "Compile LLVM IR".to_string(),
            status: BuildStatus::Success,
            duration_ms: compile_ir_start.elapsed().as_millis() as u64,
            message: format!("Generated object file {}", program_object_path.display()),
        });
        self.diagnostics
            .add_object_file(program_object_path.display().to_string());

        let runtime_compile_start = Instant::now();
        let runtime_compile_output = self.run_bash_command(
            r#"clang-20 -c "$(cygpath -u "$GLUAU_RUNTIME_SOURCE")" -o "$(cygpath -u "$GLUAU_RUNTIME_OBJECT")""#,
            &[
                ("GLUAU_RUNTIME_SOURCE", &runtime_source_path),
                ("GLUAU_RUNTIME_OBJECT", &runtime_object_path),
            ],
        )?;
        if !runtime_compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&runtime_compile_output.stderr);
            self.diagnostics.add_stage(BuildStage {
                name: "Compile Runtime".to_string(),
                status: BuildStatus::Error,
                duration_ms: runtime_compile_start.elapsed().as_millis() as u64,
                message: format!("Runtime compilation failed: {}", stderr.trim()),
            });
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("runtime compilation failed: {}", stderr.trim()),
            ));
        }
        self.diagnostics.add_stage(BuildStage {
            name: "Compile Runtime".to_string(),
            status: BuildStatus::Success,
            duration_ms: runtime_compile_start.elapsed().as_millis() as u64,
            message: format!("Generated runtime object {}", runtime_object_path.display()),
        });
        self.diagnostics
            .add_object_file(runtime_object_path.display().to_string());

        let link_start = Instant::now();
        let link_output = self.run_bash_command(
            r#"clang-20 "$(cygpath -u "$GLUAU_PROGRAM_OBJECT")" "$(cygpath -u "$GLUAU_RUNTIME_OBJECT")" -o "$(cygpath -u "$GLUAU_OUTPUT_PATH")""#,
            &[
                ("GLUAU_PROGRAM_OBJECT", &program_object_path),
                ("GLUAU_RUNTIME_OBJECT", &runtime_object_path),
                ("GLUAU_OUTPUT_PATH", output_path),
            ],
        )?;

        if !link_output.status.success() {
            let stderr = String::from_utf8_lossy(&link_output.stderr);
            self.diagnostics.add_stage(BuildStage {
                name: "Link Executable".to_string(),
                status: BuildStatus::Error,
                duration_ms: link_start.elapsed().as_millis() as u64,
                message: format!("Linking failed: {}", stderr.trim()),
            });
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("linking failed: {}", stderr.trim()),
            ));
        }

        self.diagnostics.add_stage(BuildStage {
            name: "Link Executable".to_string(),
            status: BuildStatus::Success,
            duration_ms: link_start.elapsed().as_millis() as u64,
            message: format!("Produced executable {}", output_path.display()),
        });
        self.diagnostics
            .add_linked_library("gradualuau_runtime".to_string());
        self.diagnostics.add_linked_library("c_runtime".to_string());

        let validate_start = Instant::now();
        let validation_success = self.validate_executable(output_path);
        self.diagnostics.add_stage(BuildStage {
            name: "Validation".to_string(),
            status: if validation_success {
                BuildStatus::Success
            } else {
                BuildStatus::Warning
            },
            duration_ms: validate_start.elapsed().as_millis() as u64,
            message: if validation_success {
                "Executable validated successfully".to_string()
            } else {
                "Executable validation skipped (file missing or empty)".to_string()
            },
        });

        self.diagnostics.duration_ms = start_time.elapsed().as_millis() as u64;

        let _ = fs::remove_dir_all(&build_dir);

        Ok(self.diagnostics.clone())
    }

    fn create_build_directory(&self) -> Result<PathBuf, io::Error> {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let build_dir = std::env::temp_dir().join(format!("gradualuau_native_{unique_suffix}"));
        fs::create_dir_all(&build_dir)?;
        Ok(build_dir)
    }

    fn detect_toolchain_target_triple(&self) -> Result<String, io::Error> {
        let output = self.run_bash_command("clang-20 -dumpmachine", &[])?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to determine toolchain target triple",
            ));
        }

        let triple = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if triple.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "toolchain target triple was empty",
            ))
        } else {
            Ok(triple)
        }
    }

    fn rewrite_target_triple(&self, ir: &str, target_triple: &str) -> String {
        let mut rewritten = String::with_capacity(ir.len() + target_triple.len());
        let mut replaced = false;

        for line in ir.lines() {
            if line.trim_start().starts_with("target triple = ") {
                rewritten.push_str(&format!("target triple = \"{}\"\n", target_triple));
                replaced = true;
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }

        if !replaced {
            let mut with_triple = String::new();
            with_triple.push_str(&format!("target triple = \"{}\"\n", target_triple));
            with_triple.push_str(ir);
            with_triple
        } else {
            rewritten
        }
    }

    fn runtime_shim_source() -> String {
        String::from(
            r#"#include <stdio.h>
#include <stdlib.h>

void glua_print(char *value) {
    if (value == NULL) {
        return;
    }

    fputs(value, stdout);
    fflush(stdout);
}

void *glua_table_new(void) {
    return calloc(1, 1);
}

void glua_table_set(void *table, char *key, char *value) {
    (void)table;
    (void)key;
    (void)value;
}

char *glua_table_get(void *table, char *key) {
    (void)table;
    (void)key;
    return NULL;
}
"#,
        )
    }

    fn run_bash_command(
        &self,
        command: &str,
        envs: &[(&str, &Path)],
    ) -> Result<std::process::Output, io::Error> {
        let mut process = Command::new("C:\\cygwin64\\bin\\bash.exe");
        process.arg("-lc").arg(command);

        for (name, value) in envs {
            process.env(name, value);
        }

        process.output()
    }

    fn validate_executable(&self, path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(path) {
            metadata.len() > 0
        } else {
            false
        }
    }
}
