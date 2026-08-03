use super::function::MirFunction;

#[derive(Debug, Clone)]
pub struct MirModule {
    pub name: String,
    pub functions: Vec<MirFunction>,
}

impl MirModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: Vec::new(),
        }
    }
    
    pub fn add_function(&mut self, function: MirFunction) {
        self.functions.push(function);
    }
}