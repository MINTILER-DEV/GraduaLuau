use crate::mir::types::MirType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlvmType {
    Void,
    Integer(u32),
    Float,
    Double,
    Boolean,
    Pointer(Box<LlvmType>),
    Function(Box<LlvmType>, Vec<LlvmType>),
    Unknown,
}

impl LlvmType {
    pub fn to_string(&self) -> String {
        match self {
            LlvmType::Void => "void".to_string(),
            LlvmType::Integer(bits) => format!("i{}", bits),
            LlvmType::Float => "float".to_string(),
            LlvmType::Double => "double".to_string(),
            LlvmType::Boolean => "i1".to_string(),
            LlvmType::Pointer(inner) => format!("{}*", inner.to_string()),
            LlvmType::Function(ret, params) => {
                let param_str: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                format!("{} ({})", ret.to_string(), param_str.join(", "))
            }
            LlvmType::Unknown => "i8*".to_string(),
        }
    }
}

pub fn map_mir_type(mir_type: &MirType) -> LlvmType {
    match mir_type {
        MirType::Void => LlvmType::Void,
        MirType::Integer => LlvmType::Integer(64),
        MirType::Float => LlvmType::Double,
        MirType::Boolean => LlvmType::Boolean,
        MirType::String => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
        MirType::Table => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
        MirType::Function => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
        MirType::Any => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
        MirType::Unknown => LlvmType::Unknown,
    }
}