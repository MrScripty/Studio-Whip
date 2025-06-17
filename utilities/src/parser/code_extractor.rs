use crate::config::ParserConfig;
use crate::parser::types::*;
use quote::ToTokens;
use syn::{Attribute, Expr, Lit, Meta};

pub struct CodeExtractor {
    config: ParserConfig,
}

impl CodeExtractor {
    pub fn new(config: &ParserConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
    
    pub fn extract_struct_info(&self, item_struct: &syn::ItemStruct) -> StructInfo {
        let name = item_struct.ident.to_string();
        let visibility = self.extract_visibility(&item_struct.vis);
        let documentation = self.extract_documentation(&item_struct.attrs);
        let derives = self.extract_derives(&item_struct.attrs);
        
        let fields = match &item_struct.fields {
            syn::Fields::Named(fields_named) => {
                fields_named.named.iter().map(|field| {
                    FieldInfo {
                        name: field.ident.as_ref().unwrap().to_string(),
                        field_type: field.ty.to_token_stream().to_string(),
                        visibility: self.extract_visibility(&field.vis),
                        documentation: self.extract_documentation(&field.attrs),
                    }
                }).collect()
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                fields_unnamed.unnamed.iter().enumerate().map(|(i, field)| {
                    FieldInfo {
                        name: format!("{}", i),
                        field_type: field.ty.to_token_stream().to_string(),
                        visibility: self.extract_visibility(&field.vis),
                        documentation: self.extract_documentation(&field.attrs),
                    }
                }).collect()
            }
            syn::Fields::Unit => Vec::new(),
        };
        
        // Check for Bevy-specific markers
        let is_bevy_component = self.has_bevy_derive(&derives, "Component") || 
                              self.has_bevy_derive(&derives, "Reflect");
        let is_bevy_resource = self.has_bevy_derive(&derives, "Resource");
        let is_bevy_event = self.has_bevy_derive(&derives, "Event");
        
        StructInfo {
            name,
            fields,
            derives,
            visibility,
            documentation,
            is_bevy_component,
            is_bevy_resource,
            is_bevy_event,
        }
    }
    
    pub fn extract_enum_info(&self, item_enum: &syn::ItemEnum) -> EnumInfo {
        let name = item_enum.ident.to_string();
        let visibility = self.extract_visibility(&item_enum.vis);
        let documentation = self.extract_documentation(&item_enum.attrs);
        let derives = self.extract_derives(&item_enum.attrs);
        
        let variants = item_enum.variants.iter().map(|variant| {
            let fields = match &variant.fields {
                syn::Fields::Named(fields_named) => {
                    fields_named.named.iter().map(|field| {
                        FieldInfo {
                            name: field.ident.as_ref().unwrap().to_string(),
                            field_type: field.ty.to_token_stream().to_string(),
                            visibility: self.extract_visibility(&field.vis),
                            documentation: self.extract_documentation(&field.attrs),
                        }
                    }).collect()
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    fields_unnamed.unnamed.iter().enumerate().map(|(i, field)| {
                        FieldInfo {
                            name: format!("{}", i),
                            field_type: field.ty.to_token_stream().to_string(),
                            visibility: self.extract_visibility(&field.vis),
                            documentation: self.extract_documentation(&field.attrs),
                        }
                    }).collect()
                }
                syn::Fields::Unit => Vec::new(),
            };
            
            VariantInfo {
                name: variant.ident.to_string(),
                fields,
                documentation: self.extract_documentation(&variant.attrs),
            }
        }).collect();
        
        EnumInfo {
            name,
            variants,
            derives,
            visibility,
            documentation,
        }
    }
    
    pub fn extract_function_info(&self, item_fn: &syn::ItemFn) -> FunctionInfo {
        let name = item_fn.sig.ident.to_string();
        let visibility = self.extract_visibility(&item_fn.vis);
        let documentation = self.extract_documentation(&item_fn.attrs);
        
        let parameters: Vec<ParameterInfo> = item_fn.sig.inputs.iter().map(|input| {
            match input {
                syn::FnArg::Receiver(receiver) => {
                    ParameterInfo {
                        name: "self".to_string(),
                        param_type: if receiver.reference.is_some() {
                            if receiver.mutability.is_some() {
                                "&mut self".to_string()
                            } else {
                                "&self".to_string()
                            }
                        } else {
                            "self".to_string()
                        },
                        is_self: true,
                        is_mutable: receiver.mutability.is_some(),
                    }
                }
                syn::FnArg::Typed(pat_type) => {
                    let name = match &*pat_type.pat {
                        syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                        _ => "unknown".to_string(),
                    };
                    
                    ParameterInfo {
                        name,
                        param_type: pat_type.ty.to_token_stream().to_string(),
                        is_self: false,
                        is_mutable: false, // TODO: Detect mutability
                    }
                }
            }
        }).collect();
        
        let return_type = match &item_fn.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(ty.to_token_stream().to_string()),
        };
        
        // Check if this is a Bevy system using improved detection
        let is_bevy_system = self.is_bevy_system_function(item_fn);
        
        FunctionInfo {
            name,
            parameters,
            return_type,
            visibility,
            documentation,
            is_bevy_system,
        }
    }
    
    pub fn extract_trait_info(&self, item_trait: &syn::ItemTrait) -> TraitInfo {
        let name = item_trait.ident.to_string();
        let visibility = self.extract_visibility(&item_trait.vis);
        let documentation = self.extract_documentation(&item_trait.attrs);
        
        let methods = item_trait.items.iter().filter_map(|item| {
            match item {
                syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            }
        }).collect();
        
        TraitInfo {
            name,
            methods,
            visibility,
            documentation,
        }
    }
    
    pub fn extract_impl_info(&self, item_impl: &syn::ItemImpl) -> ImplInfo {
        let target = item_impl.self_ty.to_token_stream().to_string();
        let trait_name = item_impl.trait_.as_ref().map(|(_, path, _)| {
            path.to_token_stream().to_string()
        });
        
        let methods = item_impl.items.iter().filter_map(|item| {
            match item {
                syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            }
        }).collect();
        
        ImplInfo {
            target,
            trait_name,
            methods,
        }
    }
    
    pub fn extract_use_info(&self, item_use: &syn::ItemUse) -> UseInfo {
        let path = item_use.tree.to_token_stream().to_string();
        
        // Simple extraction - could be more sophisticated
        let (path, items, is_glob) = match &item_use.tree {
            syn::UseTree::Path(use_path) => {
                let path = use_path.ident.to_string();
                // TODO: Extract nested items
                (path, Vec::new(), false)
            }
            syn::UseTree::Name(use_name) => {
                let name = use_name.ident.to_string();
                (name.clone(), vec![name], false)
            }
            syn::UseTree::Glob(_) => {
                (path.clone(), Vec::new(), true)
            }
            _ => (path.clone(), Vec::new(), false),
        };
        
        UseInfo {
            path,
            items,
            is_glob,
        }
    }
    
    pub fn extract_visibility(&self, vis: &syn::Visibility) -> Visibility {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(vis_restricted) => {
                let path = vis_restricted.path.to_token_stream().to_string();
                match path.as_str() {
                    "crate" => Visibility::Crate,
                    "super" => Visibility::Super,
                    _ => Visibility::Private,
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }
    
    fn extract_documentation(&self, attrs: &[Attribute]) -> Option<String> {
        if !self.config.include_documentation {
            return None;
        }
        
        let mut docs = Vec::new();
        
        for attr in attrs {
            if attr.path().is_ident("doc") {
                if let Ok(Meta::NameValue(meta_name_value)) = attr.meta.clone().try_into() {
                    if let Expr::Lit(expr_lit) = &meta_name_value.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            docs.push(lit_str.value());
                        }
                    }
                }
            }
        }
        
        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n"))
        }
    }
    
    fn extract_derives(&self, attrs: &[Attribute]) -> Vec<String> {
        let mut derives = Vec::new();
        
        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let Ok(Meta::List(meta_list)) = attr.meta.clone().try_into() {
                    let derive_str = meta_list.tokens.to_string();
                    // Simple parsing - split by comma and clean up
                    let derive_items: Vec<String> = derive_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    derives.extend(derive_items);
                }
            }
        }
        
        derives
    }
    
    fn has_bevy_derive(&self, derives: &[String], target: &str) -> bool {
        derives.iter().any(|derive| derive.contains(target))
    }
    
    pub fn is_bevy_plugin(&self, item_impl: &syn::ItemImpl) -> bool {
        // Check if implementing Plugin trait
        if let Some((_, path, _)) = &item_impl.trait_ {
            let trait_name = path.to_token_stream().to_string();
            if trait_name.contains("Plugin") {
                return true;
            }
        }
        false
    }
    
    fn is_bevy_system_function(&self, item_fn: &syn::ItemFn) -> bool {
        // More robust system detection
        for input in &item_fn.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                let type_str = pat_type.ty.to_token_stream().to_string();
                if type_str.contains("Query") || 
                   type_str.contains("Res<") || 
                   type_str.contains("ResMut<") ||
                   type_str.contains("Commands") ||
                   type_str.contains("EventReader") ||
                   type_str.contains("EventWriter") {
                    return true;
                }
            }
        }
        false
    }
}