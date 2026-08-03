use super::instruction::MirInstruction;
use super::types::MirBlockId;

#[derive(Debug, Clone)]
pub struct MirBasicBlock {
    pub id: MirBlockId,
    pub instructions: Vec<MirInstruction>,
    pub is_entry: bool,
    pub is_exit: bool,
}

impl MirBasicBlock {
    pub fn new(id: MirBlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
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
        self.instructions.push(instruction);
    }
}