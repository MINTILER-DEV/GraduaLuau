use super::instruction::{MirInstruction, MirTerminator};
use super::types::MirBlockId;

#[derive(Debug, Clone)]
pub struct MirBasicBlock {
    pub id: MirBlockId,
    pub instructions: Vec<MirInstruction>,
    pub terminator: Option<MirTerminator>,
    pub predecessors: Vec<MirBlockId>,
    pub successors: Vec<MirBlockId>,
    pub is_entry: bool,
    pub is_exit: bool,
}

impl MirBasicBlock {
    pub fn new(id: MirBlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            terminator: None,
            predecessors: Vec::new(),
            successors: Vec::new(),
            is_entry: false,
            is_exit: false,
        }
    }

    pub fn with_entry(id: MirBlockId) -> Self {
        let mut block = Self::new(id);
        block.is_entry = true;
        block
    }

    pub fn with_exit(id: MirBlockId) -> Self {
        let mut block = Self::new(id);
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
