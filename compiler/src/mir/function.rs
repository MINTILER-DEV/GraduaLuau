use super::block::MirBasicBlock;
use super::cfg::MirControlFlowGraph;
use super::instruction::MirInstruction;
use super::lifetime::MirLifetimeMetadata;
use super::ssa::MirSsaMetadata;
use super::types::{MirBlockId, MirFunctionId, MirType};
use super::value::{MirLocal, MirParameter, MirValueData};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub id: MirFunctionId,
    pub name: String,
    pub parameters: Vec<String>,
    pub parameter_data: Vec<MirParameter>,
    pub locals: Vec<MirLocal>,
    pub values: Vec<MirValueData>,
    pub return_type: Option<MirType>,
    pub blocks: Vec<MirBasicBlock>,
    pub entry_block: Option<usize>,
    pub exit_blocks: Vec<usize>,
    pub cfg: Option<MirControlFlowGraph>,
    pub metadata: MirFunctionMetadata,
}

impl MirFunction {
    pub fn new(id: MirFunctionId, name: String) -> Self {
        Self {
            id,
            name,
            parameters: Vec::new(),
            parameter_data: Vec::new(),
            locals: Vec::new(),
            values: Vec::new(),
            return_type: None,
            blocks: Vec::new(),
            entry_block: None,
            exit_blocks: Vec::new(),
            cfg: None,
            metadata: MirFunctionMetadata::default(),
        }
    }

    pub fn add_block(&mut self, block: MirBasicBlock) {
        self.blocks.push(block);
    }

    pub fn add_named_block(
        &mut self,
        id: MirBlockId,
        name: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> usize {
        let index = self.blocks.len();
        self.blocks.push(MirBasicBlock::with_name(id, name, span));
        index
    }

    pub fn add_instruction(&mut self, block_index: usize, instruction: MirInstruction) {
        if let Some(block) = self.blocks.get_mut(block_index) {
            block.add_instruction(instruction);
        }
    }

    pub fn add_value(&mut self, value: MirValueData) {
        self.values.push(value);
    }

    pub fn add_parameter(&mut self, parameter: MirParameter) {
        self.parameters.push(parameter.storage.clone());
        self.parameter_data.push(parameter);
    }

    pub fn add_local(&mut self, local: MirLocal) {
        if !self
            .locals
            .iter()
            .any(|existing| existing.storage == local.storage)
        {
            self.locals.push(local);
        }
    }

    pub fn is_block_terminated(&self, block_index: usize) -> bool {
        self.blocks
            .get(block_index)
            .map(|block| block.is_terminated())
            .unwrap_or(true)
    }

    pub fn rebuild_cfg(&mut self) {
        for block in &mut self.blocks {
            block.predecessors.clear();
        }

        let edges: Vec<_> = self
            .blocks
            .iter()
            .map(|block| (block.id, block.successors.clone()))
            .collect();

        for (source, successors) in edges {
            for successor in successors {
                if let Some(block) = self.blocks.iter_mut().find(|block| block.id == successor) {
                    if !block.predecessors.contains(&source) {
                        block.predecessors.push(source);
                    }
                }
            }
        }

        self.rebuild_control_flow_graph();
    }

    pub fn rebuild_control_flow_graph(&mut self) {
        self.cfg = MirControlFlowGraph::build(self);
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirFunctionMetadata {
    pub span: Option<SourceSpan>,
    pub has_explicit_return: bool,
    pub ssa: Option<MirSsaMetadata>,
    pub lifetimes: Option<MirLifetimeMetadata>,
}
