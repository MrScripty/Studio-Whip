use crate::config::ParserConfig;
use crate::parser::types::*;
use crate::parser::CodeExtractor;
use anyhow::Result;
use log::debug;
use std::path::PathBuf;
use syn::{File, visit::Visit};
use quote::ToTokens;

pub struct AstAnalyzer {
    file_path: PathBuf,
    ast: File,
    config: ParserConfig,
}

impl AstAnalyzer {
    pub fn new(file_path: &PathBuf, config: &ParserConfig) -> Result<Self> {
        let content = std::fs::read_to_string(file_path)?;
        let ast = syn::parse_file(&content)?;
        
        Ok(Self {
            file_path: file_path.clone(),
            ast,
            config: config.clone(),
        })
    }
    
    pub fn analyze(&self, extractor: &CodeExtractor) -> Result<ModuleInfo> {
        debug!("Analyzing AST for {:?}", self.file_path);
        
        let relative_path = self.file_path.to_string_lossy().to_string();
        let mut module_info = ModuleInfo::new(relative_path);
        
        // Use the visitor pattern to extract information
        let mut visitor = AstVisitor::new(extractor, &self.config);
        visitor.visit_file(&self.ast);
        
        module_info.structs = visitor.structs;
        module_info.enums = visitor.enums;
        module_info.functions = visitor.functions;
        module_info.traits = visitor.traits;
        module_info.impls = visitor.impls;
        module_info.uses = visitor.uses;
        
        // Extract Bevy-specific information
        module_info.bevy_components = visitor.bevy_components;
        module_info.bevy_resources = visitor.bevy_resources;
        module_info.bevy_systems = visitor.bevy_systems;
        module_info.bevy_events = visitor.bevy_events;
        
        Ok(module_info)
    }
}

struct AstVisitor<'a> {
    extractor: &'a CodeExtractor,
    config: &'a ParserConfig,
    
    // Collected information
    structs: Vec<StructInfo>,
    enums: Vec<EnumInfo>,
    functions: Vec<FunctionInfo>,
    traits: Vec<TraitInfo>,
    impls: Vec<ImplInfo>,
    uses: Vec<UseInfo>,
    
    // Bevy-specific
    bevy_components: Vec<ComponentInfo>,
    bevy_resources: Vec<ResourceInfo>,
    bevy_systems: Vec<SystemInfo>,
    bevy_events: Vec<EventInfo>,
}

impl<'a> AstVisitor<'a> {
    fn new(extractor: &'a CodeExtractor, config: &'a ParserConfig) -> Self {
        Self {
            extractor,
            config,
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            uses: Vec::new(),
            bevy_components: Vec::new(),
            bevy_resources: Vec::new(),
            bevy_systems: Vec::new(),
            bevy_events: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for AstVisitor<'a> {
    fn visit_item_struct(&mut self, node: &'a syn::ItemStruct) {
        if !self.config.extract_structs {
            return;
        }
        
        let visibility = self.extractor.extract_visibility(&node.vis);
        if !self.config.include_private && matches!(visibility, Visibility::Private) {
            return;
        }
        
        let struct_info = self.extractor.extract_struct_info(node);
        
        // Check if this is a Bevy component, resource, or event
        if self.config.extract_components && struct_info.is_bevy_component {
            let component_info = ComponentInfo {
                name: struct_info.name.clone(),
                fields: struct_info.fields.clone(),
                derives: struct_info.derives.clone(),
                documentation: struct_info.documentation.clone(),
            };
            self.bevy_components.push(component_info);
        }
        
        if self.config.extract_resources && struct_info.is_bevy_resource {
            let resource_info = ResourceInfo {
                name: struct_info.name.clone(),
                fields: struct_info.fields.clone(),
                derives: struct_info.derives.clone(),
                documentation: struct_info.documentation.clone(),
            };
            self.bevy_resources.push(resource_info);
        }
        
        if self.config.extract_events && struct_info.is_bevy_event {
            let event_info = EventInfo {
                name: struct_info.name.clone(),
                fields: struct_info.fields.clone(),
                derives: struct_info.derives.clone(),
                documentation: struct_info.documentation.clone(),
            };
            self.bevy_events.push(event_info);
        }
        
        self.structs.push(struct_info);
    }
    
    fn visit_item_enum(&mut self, node: &'a syn::ItemEnum) {
        if !self.config.extract_enums {
            return;
        }
        
        let visibility = self.extractor.extract_visibility(&node.vis);
        if !self.config.include_private && matches!(visibility, Visibility::Private) {
            return;
        }
        
        let enum_info = self.extractor.extract_enum_info(node);
        self.enums.push(enum_info);
    }
    
    fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
        if !self.config.extract_functions {
            return;
        }
        
        let visibility = self.extractor.extract_visibility(&node.vis);
        if !self.config.include_private && matches!(visibility, Visibility::Private) {
            return;
        }
        
        let function_info = self.extractor.extract_function_info(node);
        
        // Check if this is a Bevy system
        if self.config.extract_systems && function_info.is_bevy_system {
            let system_info = SystemInfo {
                name: function_info.name.clone(),
                parameters: function_info.parameters.clone(),
                queries: Vec::new(), // TODO: Extract from parameters
                resources: Vec::new(), // TODO: Extract from parameters
                documentation: function_info.documentation.clone(),
            };
            self.bevy_systems.push(system_info);
        }
        
        self.functions.push(function_info);
    }
    
    fn visit_item_trait(&mut self, node: &'a syn::ItemTrait) {
        if !self.config.extract_traits {
            return;
        }
        
        let visibility = self.extractor.extract_visibility(&node.vis);
        if !self.config.include_private && matches!(visibility, Visibility::Private) {
            return;
        }
        
        let trait_info = self.extractor.extract_trait_info(node);
        self.traits.push(trait_info);
    }
    
    fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
        if !self.config.extract_impls {
            return;
        }
        
        let impl_info = self.extractor.extract_impl_info(node);
        
        // Check if this is a Bevy plugin implementation
        if self.config.extract_plugins && self.extractor.is_bevy_plugin(node) {
            let target_name = node.self_ty.to_token_stream().to_string();
            let plugin_info = PluginInfo {
                name: target_name,
                systems: Vec::new(), // TODO: Extract from build method
                resources: Vec::new(),
                components: Vec::new(),
                events: Vec::new(),
            };
            // Note: We'll need to enhance this to actually extract plugin contents
        }
        
        self.impls.push(impl_info);
    }
    
    fn visit_item_use(&mut self, node: &'a syn::ItemUse) {
        let use_info = self.extractor.extract_use_info(node);
        self.uses.push(use_info);
    }
}