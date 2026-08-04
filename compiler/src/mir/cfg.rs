use std::collections::{HashMap, HashSet, VecDeque};

use super::function::MirFunction;
use super::instruction::MirTerminator;
use super::types::MirBlockId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirControlFlowGraph {
    pub entry: MirBlockId,
    pub exits: Vec<MirBlockId>,
    pub nodes: HashMap<MirBlockId, MirCfgNode>,
    pub edges: Vec<MirCfgEdge>,
    pub loops: Vec<MirLoop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirCfgNode {
    pub block_id: MirBlockId,
    pub predecessors: Vec<MirBlockId>,
    pub successors: Vec<MirBlockId>,
    pub reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirCfgEdge {
    pub source: MirBlockId,
    pub target: MirBlockId,
    pub kind: MirCfgEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirCfgEdgeKind {
    Unconditional,
    TrueBranch,
    FalseBranch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirLoop {
    pub header: MirBlockId,
    pub back_edge: MirCfgEdge,
    pub body_blocks: Vec<MirBlockId>,
    pub exit_edges: Vec<MirCfgEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirCfgValidationError {
    MissingEntry,
    InvalidEdge {
        source: MirBlockId,
        target: MirBlockId,
    },
    MissingSuccessor {
        source: MirBlockId,
        target: MirBlockId,
    },
    MissingPredecessor {
        source: MirBlockId,
        target: MirBlockId,
    },
    EntryHasPredecessor {
        entry: MirBlockId,
        predecessor: MirBlockId,
    },
}

impl MirControlFlowGraph {
    pub fn build(function: &MirFunction) -> Option<Self> {
        let entry_index = function.entry_block?;
        let entry = function.blocks.get(entry_index)?.id;
        let block_ids: HashSet<_> = function.blocks.iter().map(|block| block.id).collect();
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();

        for block in &function.blocks {
            let successors = block
                .terminator
                .as_ref()
                .map(MirTerminator::successors)
                .unwrap_or_default();
            nodes.insert(
                block.id,
                MirCfgNode {
                    block_id: block.id,
                    predecessors: Vec::new(),
                    successors: successors.clone(),
                    reachable: false,
                },
            );

            for edge in Self::edges_for_terminator(block.id, block.terminator.as_ref()) {
                if block_ids.contains(&edge.target) {
                    edges.push(edge);
                }
            }
        }

        for edge in &edges {
            if let Some(node) = nodes.get_mut(&edge.target) {
                if !node.predecessors.contains(&edge.source) {
                    node.predecessors.push(edge.source);
                }
            }
        }

        let exits = function
            .blocks
            .iter()
            .filter_map(|block| {
                matches!(block.terminator, Some(MirTerminator::Return { .. })).then_some(block.id)
            })
            .collect();

        let mut cfg = Self {
            entry,
            exits,
            nodes,
            edges,
            loops: Vec::new(),
        };
        cfg.mark_reachable();
        cfg.loops = cfg.detect_loops();
        Some(cfg)
    }

    pub fn validate(&self) -> Result<(), Vec<MirCfgValidationError>> {
        let mut errors = Vec::new();

        if !self.nodes.contains_key(&self.entry) {
            errors.push(MirCfgValidationError::MissingEntry);
        }

        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.source) || !self.nodes.contains_key(&edge.target) {
                errors.push(MirCfgValidationError::InvalidEdge {
                    source: edge.source,
                    target: edge.target,
                });
                continue;
            }

            let source_has_successor = self
                .nodes
                .get(&edge.source)
                .is_some_and(|node| node.successors.contains(&edge.target));
            if !source_has_successor {
                errors.push(MirCfgValidationError::MissingSuccessor {
                    source: edge.source,
                    target: edge.target,
                });
            }

            let target_has_predecessor = self
                .nodes
                .get(&edge.target)
                .is_some_and(|node| node.predecessors.contains(&edge.source));
            if !target_has_predecessor {
                errors.push(MirCfgValidationError::MissingPredecessor {
                    source: edge.source,
                    target: edge.target,
                });
            }
        }

        if let Some(entry_node) = self.nodes.get(&self.entry) {
            for predecessor in &entry_node.predecessors {
                errors.push(MirCfgValidationError::EntryHasPredecessor {
                    entry: self.entry,
                    predecessor: *predecessor,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn reachable_blocks(&self) -> Vec<MirBlockId> {
        self.ordered_blocks()
            .into_iter()
            .filter(|block_id| {
                self.nodes
                    .get(block_id)
                    .map(|node| node.reachable)
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn unreachable_blocks(&self) -> Vec<MirBlockId> {
        self.ordered_blocks()
            .into_iter()
            .filter(|block_id| {
                self.nodes
                    .get(block_id)
                    .map(|node| !node.reachable)
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn dfs(&self) -> Vec<MirBlockId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        self.dfs_visit(self.entry, &mut visited, &mut order);
        order
    }

    pub fn bfs(&self) -> Vec<MirBlockId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut queue = VecDeque::from([self.entry]);

        while let Some(block_id) = queue.pop_front() {
            if !visited.insert(block_id) {
                continue;
            }

            order.push(block_id);
            if let Some(node) = self.nodes.get(&block_id) {
                for successor in &node.successors {
                    queue.push_back(*successor);
                }
            }
        }

        order
    }

    pub fn post_order(&self) -> Vec<MirBlockId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        self.post_order_visit(self.entry, &mut visited, &mut order);
        order
    }

    pub fn reverse_post_order(&self) -> Vec<MirBlockId> {
        let mut order = self.post_order();
        order.reverse();
        order
    }

    pub fn dominators(&self) -> HashMap<MirBlockId, HashSet<MirBlockId>> {
        let reachable: HashSet<_> = self.reachable_blocks().into_iter().collect();
        let mut dominators = HashMap::new();

        for block_id in &reachable {
            if *block_id == self.entry {
                dominators.insert(*block_id, HashSet::from([self.entry]));
            } else {
                dominators.insert(*block_id, reachable.clone());
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block_id in reachable.iter().copied().filter(|id| *id != self.entry) {
                let predecessors: Vec<_> = self
                    .nodes
                    .get(&block_id)
                    .map(|node| {
                        node.predecessors
                            .iter()
                            .copied()
                            .filter(|predecessor| reachable.contains(predecessor))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut new_dominators = if let Some(first) = predecessors.first() {
                    dominators.get(first).cloned().unwrap_or_default()
                } else {
                    HashSet::new()
                };

                for predecessor in predecessors.iter().skip(1) {
                    if let Some(predecessor_dominators) = dominators.get(predecessor) {
                        new_dominators = new_dominators
                            .intersection(predecessor_dominators)
                            .copied()
                            .collect();
                    }
                }

                new_dominators.insert(block_id);
                if dominators.get(&block_id) != Some(&new_dominators) {
                    dominators.insert(block_id, new_dominators);
                    changed = true;
                }
            }
        }

        dominators
    }

    pub fn immediate_dominators(&self) -> HashMap<MirBlockId, MirBlockId> {
        let dominators = self.dominators();
        let mut immediate = HashMap::new();

        for (&block_id, block_dominators) in &dominators {
            if block_id == self.entry {
                continue;
            }

            let strict_dominators: Vec<_> = block_dominators
                .iter()
                .copied()
                .filter(|dominator| *dominator != block_id)
                .collect();

            for candidate in &strict_dominators {
                let dominated_by_other = strict_dominators.iter().any(|other| {
                    other != candidate
                        && dominators
                            .get(candidate)
                            .is_some_and(|candidate_doms| candidate_doms.contains(other))
                });

                if !dominated_by_other {
                    immediate.insert(block_id, *candidate);
                    break;
                }
            }
        }

        immediate
    }

    pub fn to_dot(&self, function_name: &str) -> String {
        let mut output = format!("digraph \"{}\" {{\n", function_name);

        for edge in &self.edges {
            output.push_str(&format!(
                "  Block{} -> Block{} [label=\"{}\"];\n",
                edge.source.0,
                edge.target.0,
                edge.kind.label()
            ));
        }

        output.push_str("}\n");
        output
    }

    fn edges_for_terminator(
        source: MirBlockId,
        terminator: Option<&MirTerminator>,
    ) -> Vec<MirCfgEdge> {
        match terminator {
            Some(MirTerminator::Jump { target }) => vec![MirCfgEdge {
                source,
                target: *target,
                kind: MirCfgEdgeKind::Unconditional,
            }],
            Some(MirTerminator::Branch {
                true_block,
                false_block,
                ..
            }) => vec![
                MirCfgEdge {
                    source,
                    target: *true_block,
                    kind: MirCfgEdgeKind::TrueBranch,
                },
                MirCfgEdge {
                    source,
                    target: *false_block,
                    kind: MirCfgEdgeKind::FalseBranch,
                },
            ],
            _ => Vec::new(),
        }
    }

    fn mark_reachable(&mut self) {
        let reachable = self.bfs();
        for block_id in reachable {
            if let Some(node) = self.nodes.get_mut(&block_id) {
                node.reachable = true;
            }
        }
    }

    fn detect_loops(&self) -> Vec<MirLoop> {
        let dominators = self.dominators();
        let mut loops = Vec::new();

        for edge in &self.edges {
            let is_back_edge = dominators
                .get(&edge.source)
                .is_some_and(|dominators| dominators.contains(&edge.target));

            if !is_back_edge {
                continue;
            }

            let body_blocks = self.natural_loop_blocks(edge.source, edge.target);
            let body_set: HashSet<_> = body_blocks.iter().copied().collect();
            let exit_edges = self
                .edges
                .iter()
                .filter(|candidate| {
                    body_set.contains(&candidate.source) && !body_set.contains(&candidate.target)
                })
                .cloned()
                .collect();

            loops.push(MirLoop {
                header: edge.target,
                back_edge: edge.clone(),
                body_blocks,
                exit_edges,
            });
        }

        loops
    }

    fn natural_loop_blocks(&self, latch: MirBlockId, header: MirBlockId) -> Vec<MirBlockId> {
        let mut loop_blocks = HashSet::from([header, latch]);
        let mut stack = vec![latch];

        while let Some(block_id) = stack.pop() {
            if let Some(node) = self.nodes.get(&block_id) {
                for predecessor in &node.predecessors {
                    if loop_blocks.insert(*predecessor) {
                        stack.push(*predecessor);
                    }
                }
            }
        }

        let mut ordered: Vec<_> = loop_blocks.into_iter().collect();
        ordered.sort_by_key(|block_id| block_id.0);
        ordered
    }

    fn dfs_visit(
        &self,
        block_id: MirBlockId,
        visited: &mut HashSet<MirBlockId>,
        order: &mut Vec<MirBlockId>,
    ) {
        if !visited.insert(block_id) {
            return;
        }

        order.push(block_id);
        if let Some(node) = self.nodes.get(&block_id) {
            for successor in &node.successors {
                self.dfs_visit(*successor, visited, order);
            }
        }
    }

    fn post_order_visit(
        &self,
        block_id: MirBlockId,
        visited: &mut HashSet<MirBlockId>,
        order: &mut Vec<MirBlockId>,
    ) {
        if !visited.insert(block_id) {
            return;
        }

        if let Some(node) = self.nodes.get(&block_id) {
            for successor in &node.successors {
                self.post_order_visit(*successor, visited, order);
            }
        }

        order.push(block_id);
    }

    fn ordered_blocks(&self) -> Vec<MirBlockId> {
        let mut block_ids: Vec<_> = self.nodes.keys().copied().collect();
        block_ids.sort_by_key(|block_id| block_id.0);
        block_ids
    }
}

impl MirCfgEdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unconditional => "jump",
            Self::TrueBranch => "true",
            Self::FalseBranch => "false",
        }
    }
}
