use crate::hir::HirModule;

#[derive(Debug, Clone)]
pub struct MirModule {
    pub description: String,
}

#[derive(Debug, Default)]
pub struct MirStage;

impl MirStage {
    pub fn lower(hir: &HirModule) -> MirModule {
        MirModule {
            description: format!("MIR stub derived from HIR module: {} with {} functions and {} global variables", 
                hir.name, hir.functions.len(), hir.global_variables.len()),
        }
    }
}
