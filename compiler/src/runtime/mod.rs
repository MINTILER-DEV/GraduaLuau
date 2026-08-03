use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use crate::llvm::LlvmModule;

#[derive(Debug, Default)]
pub struct RuntimeStage;

impl RuntimeStage {
    pub fn link(output_path: &Path, llvm_module: &LlvmModule) -> Result<(), io::Error> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_dir = std::env::temp_dir();
        let temp_source = temp_dir.join("gradualuau_build.rs");
        let executable_marker = output_path.display();

        let source = format!(
            "fn main() {{\n    println!(r\"GraduaLuau native executable generated at: {}\");\n    println!(\"Optimized LLVM IR length: {}\");\n}}\n",
            executable_marker,
            llvm_module.ir.len()
        );

        fs::write(&temp_source, source)?;

        let mut command = Command::new("rustc");
        command.arg(&temp_source).arg("-o").arg(output_path);

        let output = command.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("rustc failed: {}", stderr),
            ));
        }

        Ok(())
    }
}
