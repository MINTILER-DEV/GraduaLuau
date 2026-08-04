use super::function::MirFunction;
use super::value::{MirConstant, MirGlobal};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct MirModule {
    pub name: String,
    pub functions: Vec<MirFunction>,
    pub globals: Vec<MirGlobal>,
    pub constants: Vec<MirConstant>,
    pub metadata: MirModuleMetadata,
}

impl MirModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: Vec::new(),
            globals: Vec::new(),
            constants: Vec::new(),
            metadata: MirModuleMetadata::default(),
        }
    }

    pub fn add_function(&mut self, function: MirFunction) {
        self.functions.push(function);
    }

    pub fn add_global(&mut self, global: MirGlobal) {
        self.globals.push(global);
    }

    pub fn add_constant(&mut self, constant: MirConstant) {
        self.constants.push(constant);
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirModuleMetadata {
    pub span: Option<SourceSpan>,
    pub root_scope: Option<usize>,
}
