use crate::mir::MirModule;

#[derive(Debug, Clone)]
pub struct LlvmModule {
    pub ir: String,
}

#[derive(Debug, Default)]
pub struct LlvmStage;

impl LlvmStage {
    pub fn generate(mir: &MirModule) -> LlvmModule {
        let mut ir = String::new();
        ir.push_str("; LLVM IR generated from MIR\n");
        ir.push_str(&format!("; Module: {}\n", mir.name));
        ir.push_str(&format!("; Functions: {}\n", mir.functions.len()));
        
        for function in &mir.functions {
            ir.push_str(&format!("; Function: {} with {} blocks\n", function.name, function.blocks.len()));
        }
        
        LlvmModule { ir }
    }
}
