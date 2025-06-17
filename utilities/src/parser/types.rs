use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub modules: HashMap<String, ModuleInfo>,
    pub dependencies: Vec<Dependency>,
    pub bevy_info: BevyInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub path: String,
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub functions: Vec<FunctionInfo>,
    pub traits: Vec<TraitInfo>,
    pub impls: Vec<ImplInfo>,
    pub bevy_components: Vec<ComponentInfo>,
    pub bevy_resources: Vec<ResourceInfo>,
    pub bevy_systems: Vec<SystemInfo>,
    pub bevy_events: Vec<EventInfo>,
    pub uses: Vec<UseInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub is_bevy_component: bool,
    pub is_bevy_resource: bool,
    pub is_bevy_event: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<VariantInfo>,
    pub derives: Vec<String>,
    pub visibility: Visibility,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub is_bevy_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitInfo {
    pub name: String,
    pub methods: Vec<String>,
    pub visibility: Visibility,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplInfo {
    pub target: String,
    pub trait_name: Option<String>,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub queries: Vec<String>,
    pub resources: Vec<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: String,
    pub visibility: Visibility,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: String,
    pub is_self: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseInfo {
    pub path: String,
    pub items: Vec<String>,
    pub is_glob: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from_module: String,
    pub to_module: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BevyInfo {
    pub plugins: Vec<PluginInfo>,
    pub app_structure: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub systems: Vec<String>,
    pub resources: Vec<String>,
    pub components: Vec<String>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Use,
    FunctionCall,
    StructField,
    TraitImpl,
}

impl ParseResult {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependencies: Vec::new(),
            bevy_info: BevyInfo {
                plugins: Vec::new(),
                app_structure: Vec::new(),
            },
        }
    }
    
    pub fn add_file_result(&mut self, module_path: String, module_info: ModuleInfo) {
        self.modules.insert(module_path, module_info);
    }
    
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str("# Architecture Overview\n\n");
        
        // Summary
        md.push_str("## Project Summary\n\n");
        md.push_str(&format!("- **Total Modules**: {}\n", self.modules.len()));
        md.push_str(&format!("- **Bevy Plugins**: {}\n", self.bevy_info.plugins.len()));
        
        // Count totals
        let total_components: usize = self.modules.values().map(|m| m.bevy_components.len()).sum();
        let total_systems: usize = self.modules.values().map(|m| m.bevy_systems.len()).sum();
        let total_resources: usize = self.modules.values().map(|m| m.bevy_resources.len()).sum();
        let total_events: usize = self.modules.values().map(|m| m.bevy_events.len()).sum();
        
        md.push_str(&format!("- **ECS Components**: {}\n", total_components));
        md.push_str(&format!("- **Systems**: {}\n", total_systems));
        md.push_str(&format!("- **Resources**: {}\n", total_resources));
        md.push_str(&format!("- **Events**: {}\n\n", total_events));
        
        // Core Architecture Components
        md.push_str("## Core Architecture\n\n");
        
        // Group modules by category
        let mut rendering_modules = Vec::new();
        let mut plugin_modules = Vec::new();
        let mut component_modules = Vec::new();
        let mut other_modules = Vec::new();
        
        for (path, module) in &self.modules {
            if path.contains("rendering") {
                rendering_modules.push((path, module));
            } else if path.contains("plugins") || module.structs.iter().any(|s| s.name.contains("Plugin")) {
                plugin_modules.push((path, module));
            } else if !module.bevy_components.is_empty() || path.contains("components") {
                component_modules.push((path, module));
            } else {
                other_modules.push((path, module));
            }
        }
        
        // Rendering System
        if !rendering_modules.is_empty() {
            md.push_str("### Rendering System\n\n");
            for (path, module) in rendering_modules {
                md.push_str(&format!("**{}**\n", path));
                if !module.structs.is_empty() {
                    let key_structs: Vec<&str> = module.structs.iter()
                        .filter(|s| !s.name.ends_with("Resource"))
                        .map(|s| s.name.as_str())
                        .collect();
                    if !key_structs.is_empty() {
                        md.push_str(&format!("- Core Types: {}\n", key_structs.join(", ")));
                    }
                }
                if !module.bevy_systems.is_empty() {
                    let systems: Vec<&str> = module.bevy_systems.iter().map(|s| s.name.as_str()).collect();
                    md.push_str(&format!("- Systems: {}\n", systems.join(", ")));
                }
                md.push_str("\n");
            }
        }
        
        // Plugin System
        if !plugin_modules.is_empty() {
            md.push_str("### Plugin Architecture\n\n");
            for (path, module) in plugin_modules {
                md.push_str(&format!("**{}**\n", path));
                let plugins: Vec<&str> = module.structs.iter()
                    .filter(|s| s.name.contains("Plugin"))
                    .map(|s| s.name.as_str())
                    .collect();
                if !plugins.is_empty() {
                    md.push_str(&format!("- Plugins: {}\n", plugins.join(", ")));
                }
                if !module.bevy_systems.is_empty() {
                    let systems: Vec<&str> = module.bevy_systems.iter().map(|s| s.name.as_str()).collect();
                    md.push_str(&format!("- Systems: {}\n", systems.join(", ")));
                }
                md.push_str("\n");
            }
        }
        
        // ECS Components
        if !component_modules.is_empty() {
            md.push_str("### ECS Component System\n\n");
            for (path, module) in component_modules {
                if !module.bevy_components.is_empty() {
                    md.push_str(&format!("**{}**\n", path));
                    let components: Vec<&str> = module.bevy_components.iter().map(|c| c.name.as_str()).collect();
                    md.push_str(&format!("- Components: {}\n\n", components.join(", ")));
                }
            }
        }
        
        // Resources and Events
        let all_resources: Vec<&str> = self.modules.values()
            .flat_map(|m| m.bevy_resources.iter().map(|r| r.name.as_str()))
            .collect();
        let all_events: Vec<&str> = self.modules.values()
            .flat_map(|m| m.bevy_events.iter().map(|e| e.name.as_str()))
            .collect();
            
        if !all_resources.is_empty() {
            md.push_str("### Global Resources\n\n");
            md.push_str(&format!("{}\n\n", all_resources.join(", ")));
        }
        
        if !all_events.is_empty() {
            md.push_str("### Event Types\n\n");
            md.push_str(&format!("{}\n\n", all_events.join(", ")));
        }
        
        // All Systems Summary
        let all_systems: Vec<&str> = self.modules.values()
            .flat_map(|m| m.bevy_systems.iter().map(|s| s.name.as_str()))
            .collect();
        if !all_systems.is_empty() {
            md.push_str("### System Functions\n\n");
            for system in all_systems {
                md.push_str(&format!("- `{}`\n", system));
            }
            md.push_str("\n");
        }
        
        md
    }
}

impl ModuleInfo {
    pub fn new(path: String) -> Self {
        Self {
            path,
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            bevy_components: Vec::new(),
            bevy_resources: Vec::new(),
            bevy_systems: Vec::new(),
            bevy_events: Vec::new(),
            uses: Vec::new(),
        }
    }
}