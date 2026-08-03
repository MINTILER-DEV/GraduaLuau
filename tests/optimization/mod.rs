use compiler::optimization::{OptimizationStage, OptimizationLevel, OptimizationPasses};
use compiler::llvm::LlvmModule;

#[test]
fn test_optimization_levels() {
    let o0 = OptimizationLevel::from_string("0").unwrap();
    assert_eq!(o0, OptimizationLevel::O0);
    
    let o1 = OptimizationLevel::from_string("O1").unwrap();
    assert_eq!(o1, OptimizationLevel::O1);
    
    let o2 = OptimizationLevel::from_string("2").unwrap();
    assert_eq!(o2, OptimizationLevel::O2);
    
    let o3 = OptimizationLevel::from_string("O3").unwrap();
    assert_eq!(o3, OptimizationLevel::O3);
}

#[test]
fn test_invalid_optimization_level() {
    let result = OptimizationLevel::from_string("O4");
    assert!(result.is_err());
}

#[test]
fn test_optimization_level_strings() {
    assert_eq!(OptimizationLevel::O0.as_str(), "O0");
    assert_eq!(OptimizationLevel::O1.as_str(), "O1");
    assert_eq!(OptimizationLevel::O2.as_str(), "O2");
    assert_eq!(OptimizationLevel::O3.as_str(), "O3");
}

#[test]
fn test_default_optimization_level() {
    let default = OptimizationLevel::default();
    assert_eq!(default, OptimizationLevel::O2);
}

#[test]
fn test_optimization_passes_o0() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O0);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(optimized.contains("Optimized at level O0"));
}

#[test]
fn test_optimization_passes_o1() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O1);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(optimized.contains("Optimized at level O1"));
}

#[test]
fn test_optimization_passes_o2() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O2);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(optimized.contains("Optimized at level O2"));
}

#[test]
fn test_optimization_passes_o3() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O3);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(optimized.contains("Optimized at level O3"));
}

#[test]
fn test_dead_code_elimination() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ; dead code\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O1);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.contains("dead code"));
}

#[test]
fn test_control_flow_simplification() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ; unreachable\n    ret i32 0\n}";
    let passes = OptimizationPasses::new(OptimizationLevel::O3);
    let result = passes.run(llvm_ir);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.contains("unreachable"));
}

#[test]
fn test_optimization_stage() {
    let llvm_ir = "define i32 @main() {\nentry:\n    ret i32 0\n}";
    let llvm_module = LlvmModule { ir: llvm_ir.to_string() };
    
    let stage = OptimizationStage::new(OptimizationLevel::O2);
    let result = stage.optimize(&llvm_module);
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(optimized.ir.contains("Optimized at level O2"));
}