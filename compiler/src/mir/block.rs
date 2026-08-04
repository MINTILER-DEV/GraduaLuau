use super::instruction::{MirInstruction, MirTerminator};
use super::types::MirBlockId;
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct MirBasicBlock {
    pub id: MirBlockId,
    pub name: String,
    pub instructions: Vec<MirInstruction>,
    pub terminator: Option<MirTerminator>,
    pub predecessors: Vec<MirBlockId>,
    pub successors: Vec<MirBlockId>,
    pub span: Option<SourceSpan>,
    pub is_entry: bool,
    pub is_exit: bool,
}

impl MirBasicBlock {
    pub fn new(id: MirBlockId) -> Self {
        Self {
            id,
            name: format!("Block{}", id.0),
            instructions: Vec::new(),
            terminator: None,
            predecessors: Vec::new(),
            successors: Vec::new(),
            span: None,
            is_entry: false,
            is_exit: false,
        }
    }

    pub fn with_name(id: MirBlockId, name: impl Into<String>, span: Option<SourceSpan>) -> Self {
        let mut block = Self::new(id);
        block.name = name.into();
        block.span = span;
        block
    }

    pub fn with_entry(id: MirBlockId) -> Self {
        let mut block = Self::with_name(id, "entry", None);
        block.is_entry = true;
        block
    }

    pub fn with_exit(id: MirBlockId) -> Self {
        let mut block = Self::with_name(id, "exit", None);
        block.is_exit = true;
        block
    }

    pub fn add_instruction(&mut self, instruction: MirInstruction) {
        if instruction.is_terminator() {
            self.add_terminator(instruction);
            return;
        }

        self.instructions.push(instruction);
    }

    pub fn add_terminator(&mut self, instruction: MirInstruction) {
        if let Some(terminator) = instruction.terminator() {
            self.successors = terminator.successors();
            self.terminator = Some(terminator);
        }

        self.instructions.push(instruction);
    }

    pub fn is_terminated(&self) -> bool {
        self.terminator.is_some()
    }
}
