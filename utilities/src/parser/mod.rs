mod ast_analyzer;
mod code_extractor;
mod types;

pub use ast_analyzer::AstAnalyzer;
pub use code_extractor::CodeExtractor;
pub use types::*;

use crate::config::ParserConfig;
use anyhow::Result;
use log::{debug, info};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct CodeParser {
    project_path: PathBuf,
    config: ParserConfig,
}

impl CodeParser {
    pub fn new(project_path: PathBuf, config: ParserConfig) -> Result<Self> {
        if !project_path.exists() {
            anyhow::bail!("Project path does not exist: {:?}", project_path);
        }
        
        let src_path = project_path.join("src");
        if !src_path.exists() {
            anyhow::bail!("Source directory not found: {:?}", src_path);
        }
        
        Ok(Self {
            project_path,
            config,
        })
    }
    
    pub fn parse(&self) -> Result<ParseResult> {
        info!("Starting code analysis for {:?}", self.project_path);
        
        let src_path = self.project_path.join("src");
        let mut result = ParseResult::new();
        
        // Find all Rust files
        let rust_files: Vec<PathBuf> = WalkDir::new(&src_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext == "rs")
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        
        info!("Found {} Rust files to analyze", rust_files.len());
        
        // Analyze each file
        for file_path in rust_files {
            debug!("Analyzing file: {:?}", file_path);
            
            // Calculate relative path from src directory
            let relative_path = file_path.strip_prefix(&src_path)?;
            let module_path = self.path_to_module_name(relative_path);
            
            // Check if this module should be included
            if self.should_skip_module(&module_path) {
                debug!("Skipping module: {}", module_path);
                continue;
            }
            
            let analyzer = AstAnalyzer::new(&file_path, &self.config)?;
            let extractor = CodeExtractor::new(&self.config);
            
            let file_result = analyzer.analyze(&extractor)?;
            result.add_file_result(module_path, file_result);
        }
        
        info!("Analysis complete. Found {} modules", result.modules.len());
        Ok(result)
    }
    
    fn path_to_module_name(&self, path: &std::path::Path) -> String {
        let mut parts: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();
        
        // Remove .rs extension from the last part
        if let Some(last) = parts.last_mut() {
            if last.ends_with(".rs") {
                *last = last.trim_end_matches(".rs").to_string();
            }
        }
        
        // Convert mod.rs to parent directory name
        if parts.last() == Some(&"mod".to_string()) && parts.len() > 1 {
            parts.pop();
        }
        
        parts.join("::")
    }
    
    fn should_skip_module(&self, module_path: &str) -> bool {
        // Check exclusions
        if self.config.exclude_modules.contains(module_path) {
            return true;
        }
        
        // Check inclusions (if specified)
        if let Some(ref include_modules) = self.config.include_modules {
            if !include_modules.contains(module_path) {
                return true;
            }
        }
        
        false
    }
}