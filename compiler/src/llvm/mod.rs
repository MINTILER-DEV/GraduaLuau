use crate::mir::MirModule;

#[derive(Debug, Clone)]
pub struct LlvmModule {
    pub ir: String,
}

#[derive(Debug, Default)]
pub struct LlvmStage;

impl LlvmStage {
    pub fn generate(mir: &MirModule) -> LlvmModule {
        LlvmModule {
            ir: format!("; LLVM IR stub\n; {}\n", mir.description),
        }
    }
}
