use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Include only these modules (if specified)
    pub include_modules: Option<HashSet<String>>,
    
    /// Exclude these modules
    pub exclude_modules: HashSet<String>,
    
    /// Include private items (false = public only)
    pub include_private: bool,
    
    /// Types of items to extract
    pub extract_structs: bool,
    pub extract_enums: bool,
    pub extract_functions: bool,
    pub extract_impls: bool,
    pub extract_traits: bool,
    pub extract_modules: bool,
    
    /// Bevy-specific extractions
    pub extract_components: bool,
    pub extract_resources: bool,
    pub extract_systems: bool,
    pub extract_plugins: bool,
    pub extract_events: bool,
    
    /// Analysis depth
    pub analyze_dependencies: bool,
    pub analyze_function_calls: bool,
    
    /// Output configuration
    pub include_documentation: bool,
    pub include_source_locations: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            include_modules: None,
            exclude_modules: HashSet::new(),
            include_private: false,
            extract_structs: true,
            extract_enums: true,
            extract_functions: true,
            extract_impls: true,
            extract_traits: true,
            extract_modules: true,
            extract_components: true,
            extract_resources: true,
            extract_systems: true,
            extract_plugins: true,
            extract_events: true,
            analyze_dependencies: true,
            analyze_function_calls: false, // Expensive operation
            include_documentation: true,
            include_source_locations: true,
        }
    }
}

impl ParserConfig {
    /// Configuration optimized for high-level architecture documentation
    pub fn architecture_default() -> Self {
        Self {
            include_modules: None,
            exclude_modules: HashSet::new(),
            include_private: false, // Public API only
            extract_structs: true,
            extract_enums: false,
            extract_functions: false, // Too detailed for architecture
            extract_impls: false,
            extract_traits: true,
            extract_modules: true,
            extract_components: true,
            extract_resources: true,
            extract_systems: true,
            extract_plugins: true,
            extract_events: true,
            analyze_dependencies: true,
            analyze_function_calls: false,
            include_documentation: true,
            include_source_locations: false, // Not needed for high-level view
        }
    }
    
    /// Load configuration from TOML file
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub fn to_file(&self, path: PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}