use std::{collections::HashSet, vec};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FieldsNamed, GenericParam, Ident, ItemImpl, ItemStruct, Token, WherePredicate, parse::{Parse, ParseStream}, punctuated::Punctuated, token::Comma};

use crate::{space::{Space, SpaceAbrreviation, SpaceName}, transition::TransitionArgs};

// Possible state spaces along with defaults and the former struct
pub struct TypeState {
    spaces: Punctuated<Space, Token![;]>,
    input_struct: ItemStruct,
    impl_blocks: Vec<ValidatedImpl>
}

pub struct ValidatedImpl {
    pub original_impl: ItemImpl,
    pub transitions: Vec<(usize, TransitionArgs)>
}

impl Parse for TypeState {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut spaces = Punctuated::new();

        // Stop parsing spaces when we hit `struct` or `pub struct`
        while !input.is_empty() {
            if input.peek(Token![struct]) || (input.peek(Token![pub]) && input.peek2(Token![struct])) {
                break;
            }

            let space = Space::parse(input)?;
            spaces.push_value(space);

            if input.peek(Token![;]) {
                let punct: Token![;] = input.parse()?;
                spaces.push_punct(punct);
            } else {
                break;
            }
        }

        let input_struct = input.parse::<ItemStruct>()?;

        let mut impl_blocks = Vec::new();
        while !input.is_empty() {
            let item_impl = input.parse::<ItemImpl>()?;
            let mut transitions = Vec::new();

            for (index, item) in item_impl.items.iter().enumerate() {
                if let syn::ImplItem::Fn(method) = item {
                    for attr in &method.attrs {
                        if attr.path().is_ident("transition") {
                            let args = TransitionArgs::parse(attr)?;
                            transitions.push((index, args));
                        }
                    }
                }
            }

            impl_blocks.push(ValidatedImpl {
                original_impl: item_impl,
                transitions,
            });
        }

        let mut abbreviations: HashSet<SpaceAbrreviation> = HashSet::new();
        let mut names: HashSet<SpaceName> = HashSet::new();

        for space in &spaces {
            let abbr_span = space.abbreviation.span();
            let abbr = &space.abbreviation;
            
            let name_span = space.name.span();
            let name = &space.name;

            // ensuring uniqueness
            if !abbreviations.insert(abbr.clone()) {
                return Err(syn::Error::new(abbr_span, "collision of abbreviations"));
            }
            if !names.insert(name.clone()) {
                return Err(syn::Error::new(name_span, "names must be unique"));
            }
        }

        Ok(Self {
            spaces,
            input_struct,
            impl_blocks,
        })
    }
}

impl ToTokens for TypeState {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let spaces = &self.spaces;
        let mut new_struct = self.input_struct.clone();
        
        new_struct.vis = syn::parse_quote! { pub };

        // For the new struct
        let mut where_clause_pred: Punctuated<WherePredicate, Comma> = Punctuated::new();
        let mut generics: Punctuated<GenericParam, Comma>  = Punctuated::new();
        let mut fields: Punctuated<syn::Field, Comma> = Punctuated::new();

        for space in spaces {
            let space_name = &space.name;
            let marker_trait_name = ident_join(
                space_name,
                &capitalize_first(space_name.to_string()),
                "State",
            );

            let marker_trait_fn = ident_join(
                space_name,
                &space_name.to_string().to_lowercase(),
                "_state",
            );

            let space_abbr = &space.abbreviation;
            let rc_state_field = marker_trait_fn.clone();

            let mut space_tokens = TokenStream::new();

            // Define the marker trait inside the space module
            space_tokens.extend(quote! {
                pub trait #marker_trait_name {
                    fn #marker_trait_fn(&self) -> &'static str; 
                }
            });

            for state in &space.states {
                let mut modified_state = state.clone();
                
                // pub on the struct field
                modified_state.visibility = syn::Visibility::Public(syn::token::Pub::default());
                
                // named fields
                if let Some(ref mut fields) = modified_state.fields {
                    for f in fields {
                        if matches!(f.visibility, Some(syn::Visibility::Inherited)) {
                            f.visibility = Some(syn::parse_quote! { pub });
                        }
                    }
                }

                // tuple/unnamed
                if let Some(ref mut tuple_fields) = modified_state.tuple_fields {
                    for f in tuple_fields {
                        if matches!(f.vis, syn::Visibility::Inherited) {
                            f.vis = syn::parse_quote! { pub };
                        }
                    }
                }

                let state_name = &state.name;
                space_tokens.extend(quote! {
                    #[derive(Debug, Copy, Clone)]
                    #modified_state

                    impl #marker_trait_name for #state_name {
                        fn #marker_trait_fn(&self) -> &'static str {
                            stringify!(#state_name)
                        }
                    }
                });
            }

            // space <=> module
            tokens.extend(quote! {
                pub mod #space_name {
                    use super::*;
                    #space_tokens
                }
            });

            if let Some(ref def) = space.default {
                let type_path = match def {
                    syn::Expr::Path(expr_path) => &expr_path.path,
                    syn::Expr::Struct(expr_struct) => &expr_struct.path,
                    _ => {
                        let err = syn::Error::new_spanned(def, "Unsupported default expression");
                        tokens.extend(err.to_compile_error());
                        return;
                    }
                };

                // mod name
                generics.push(syn::parse_quote!(#space_abbr = #space_name::#type_path));
            } else {
                generics.push(syn::parse_quote!(#space_abbr));
            }

            // trait bound with the space name
            where_clause_pred.push(syn::parse_quote!(#space_abbr: #space_name::#marker_trait_name));
            fields.push(syn::parse_quote!(pub #rc_state_field: std::rc::Rc<#space_abbr>));
        }

        // mutate the struct directly
        new_struct.generics.params.extend(generics);
        let where_clause = new_struct.generics.make_where_clause();
        where_clause.predicates.extend(where_clause_pred);

        match new_struct.fields {
            syn::Fields::Named(ref mut existing_fields) => {
                existing_fields.named.extend(fields);
            },
            syn::Fields::Unnamed(ref mut existing_fields) => {
                existing_fields.unnamed.extend(fields);
            },
            syn::Fields::Unit => {
                new_struct.semi_token = None;
                new_struct.fields = syn::Fields::Named(FieldsNamed {
                    brace_token: Default::default(),
                    named: fields,
                });
            },
        }

        let struct_name = &self.input_struct.ident;
        let space_abrs: Vec<_> = self.spaces.iter().map(|s| &s.abbreviation).collect();
        let struct_where = &new_struct.generics.where_clause;

        let mut generated_impls = TokenStream::new();

        for val_impl in &self.impl_blocks {
            for (method_idx, args) in &val_impl.transitions {
                let mut modified_impl_item = val_impl.original_impl.items[*method_idx].clone();

                if let syn::ImplItem::Fn(method) = &mut modified_impl_item {
                    method.attrs.retain(|attr| !attr.path().is_ident("transition"));

                    let return_type = build_struct_type_with_state(
                        struct_name, 
                        &self.spaces, 
                        &args.space_name, 
                        &args.to,
                        true
                    );

                    method.sig.output = syn::parse_quote! { -> #return_type };
                }

                let target_space_name = &args.space_name;
                let from_state = &args.from;

                let transitioning_space = self.spaces.iter()
                    .find(|s| s.name == SpaceName(target_space_name.clone().0))
                    .unwrap();
                let transitioning_abbr = &transitioning_space.abbreviation;

                let impl_generics: Vec<_> = self.spaces.iter().filter_map(|s| {
                    if s.name == SpaceName(target_space_name.clone().0) {
                        None
                    } else {
                        let abbr = &s.abbreviation;
                        Some(quote! { #abbr })
                    }
                }).collect();

                let self_type = build_struct_type_with_state(
                    struct_name,
                    &self.spaces,
                    target_space_name,
                    from_state,
                    true,
                );

                let filtered_predicates: Vec<_> = new_struct.generics.where_clause.as_ref()
                    .map(|wc| {
                        wc.predicates.iter().filter(|pred| {
                            let pred_str = quote!(#pred).to_string();
                            !pred_str.contains(&transitioning_abbr.to_string())
                        }).collect()
                    })
                    .unwrap_or_default();

                let filtered_where = if filtered_predicates.is_empty() {
                    quote! {}
                } else {
                    quote! { where #(#filtered_predicates),* }
                };

                generated_impls.extend(quote! {
                    impl<#(#impl_generics),*> #self_type #filtered_where {
                        #modified_impl_item
                    }
                });
            }
        }

        let mut transition_helpers = TokenStream::new();
        
        let mut trait_impls = TokenStream::new();
    
        let new_args = match &self.input_struct.fields {
            syn::Fields::Named(fields) => {
                fields.named.iter().map(|f|{
                    let ident = f.ident.as_ref().unwrap();
                    let ty = f.ty.clone();

                    quote! {#ident: #ty}
                }).collect::<Vec<_>>()
            }
            _ => vec![],
        };

        let inits = match &self.input_struct.fields {
            syn::Fields::Named(fields) => {
                fields.named.iter().map(|f|{
                    let ident = f.ident.as_ref().unwrap();

                    quote! {#ident}
                }).collect::<Vec<_>>()
            }
            _ => vec![],
        };

        for space in &self.spaces {
            let space_name = &space.name;
            let transition_fn_name = Ident::new(&format!("transition_{}", space_name), space_name.span());
            
            let return_type = build_struct_type_with_state(
                struct_name,
                &self.spaces,
                space_name,
                &syn::parse_quote!(N),
                false, 
            );

            let original_field_inits = match &self.input_struct.fields {
                syn::Fields::Named(fields) => {
                    fields.named.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        quote! { #ident: self.#ident }
                    }).collect::<Vec<_>>()
                }
                _ => vec![],
            };

            let space_field_inits = self.spaces.iter().map(|s| {
                let f_name = ident_join(&s.name, &s.name.to_string().to_lowercase(), "_state");
                if s.name == *space_name {
                    quote! { #f_name: std::rc::Rc::new(next) }
                } else {
                    quote! { #f_name: self.#f_name }
                }
            });

            let marker_trait_name = ident_join(
                space_name,
                &capitalize_first(space_name.to_string()),
                "State",
            );

            let marker_trait_fn = ident_join(
                &Ident::new(&space_name.to_string().to_lowercase(), space_name.span()),
                &space_name.to_string().to_lowercase(),
                "_state",
            );

            let rc_state_field = marker_trait_fn.clone();

            transition_helpers.extend(quote! {
                pub fn #transition_fn_name<N: #space_name::#marker_trait_name + 'static>(self, next: N) -> #return_type {
                    #struct_name {
                        #(#original_field_inits,)*
                        #(#space_field_inits,)*
                    }
                }
            });

            trait_impls.extend(quote! {
                impl<#(#space_abrs),*> #space_name::#marker_trait_name for #struct_name<#(#space_abrs),*> #struct_where {
                    fn #marker_trait_fn(&self) -> &'static str {
                        self.#rc_state_field.#marker_trait_fn()
                    }
                }
            });
        }

        // Default state initializers for `new()` constructor
        let default_inits = self.spaces.iter().map(|space| {
            let fn_field = ident_join(&space.name, &space.name.to_string().to_lowercase(), "_state");
            let space_name = &space.name;
            if let Some(ref def) = space.default {
                quote! { #fn_field: std::rc::Rc::new(#space_name::#def) }
            } else {
                quote! { #fn_field: std::rc::Rc::new(Default::default()) }
            }
        });

        // append
        tokens.extend(quote! {
            #new_struct

            impl<#(#space_abrs),*> #struct_name<#(#space_abrs),*> #struct_where {
                #transition_helpers
            }

            impl #struct_name {
                pub fn new(#(#new_args),*) -> Self {
                    #struct_name {
                        #(#inits,)*
                        #(#default_inits),*
                    }
                }
            }

            #trait_impls
            
            #generated_impls
        });
    }
}

pub fn build_struct_type_with_state(
    struct_name: &Ident,
    spaces: &Punctuated<Space, Token![;]>,
    target_space_name: &Ident,
    substitute_state: &Ident,
    qualify: bool,
) -> TokenStream {
    let type_args: Vec<_> = spaces.iter().map(|space| {
        if space.name == SpaceName(target_space_name.clone()) {
            if qualify {
                quote! { #target_space_name::#substitute_state }
            } else {
                quote! { #substitute_state }
            }
        } else {
            let abbr = &space.abbreviation;
            quote! { #abbr }
        }
    }).collect();

    quote! {
        #struct_name<#(#type_args),*>
    }
}

// helper functions
pub fn ident_join(ident: &Ident, prefix: &str, suffix: &str) -> Ident {
    Ident::new(&format!("{}{}", prefix, suffix), ident.span())
}

pub fn capitalize_first(s: String) -> String {
    let mut chars = s.chars();

    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}