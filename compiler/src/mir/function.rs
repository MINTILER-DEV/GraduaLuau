use super::block::MirBasicBlock;
use super::types::{MirFunctionId, MirType};
use super::instruction::MirInstruction;

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub id: MirFunctionId,
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<MirType>,
    pub blocks: Vec<MirBasicBlock>,
    pub entry_block: Option<usize>,
    pub exit_blocks: Vec<usize>,
}

impl MirFunction {
    pub fn new(id: MirFunctionId, name: String) -> Self {
        Self {
            id,
            name,
            parameters: Vec::new(),
            return_type: None,
            blocks: Vec::new(),
            entry_block: None,
            exit_blocks: Vec::new(),
        }
    }
    
    pub fn add_block(&mut self, block: MirBasicBlock) {
        self.blocks.push(block);
    }
    
    pub fn add_instruction(&mut self, block_index: usize, instruction: MirInstruction) {
        if let Some(block) = self.blocks.get_mut(block_index) {
            block.add_instruction(instruction);
        }
    }
}