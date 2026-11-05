use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Visibility, braced, parenthesized};
use syn::{Ident, Token, Type, parse::{Parse, ParseStream}, punctuated::Punctuated, token};

use crate::state_field::StateField;
// Represents a state, possibly with fields
#[derive(Clone)]
pub struct State {
    pub name: Ident,
    pub visibility: Visibility,
    pub attrs: Vec<Attribute>,
    // braces stored for later implementing `ToTokens`
    pub braces: Option<token::Brace>,
    pub fields: Option<Punctuated<StateField, Token![,]>>,
    // tuple structs
    pub parens: Option<token::Paren>,
    pub tuple_fields: Option<Punctuated<syn::Field, Token![,]>>,
    // shared
    pub shared: bool
}

impl Parse for State {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let visibility = input.parse::<Visibility>()?;
        let name = input.parse::<Ident>()?;

        let mut shared = false;
        attrs.retain(|attr| {
            if attr.path().is_ident("shared") {
                shared = true;
                false // remove it
            } else {
                true // keep other attributes like #[derive(...)]
            }
        });

        let mut braces = None;
        let mut fields = None;
        let mut parens = None;
        let mut tuple_fields = None;

        // if there are braces -> there are fields to parse
        if input.peek(token::Brace) {
            let content;
            braces = Some(braced!(content in input));
            fields = Some(content.parse_terminated(StateField::parse, Token![,])?);
        } else if input.peek(token::Paren) {
            let content;
            parens = Some(parenthesized!(content in input));
            tuple_fields = Some(content.parse_terminated(syn::Field::parse_unnamed, Token![,])?);
        }

        Ok(Self {
            name,
            braces,
            fields,
            parens,
            tuple_fields,
            visibility,
            attrs,
            shared
        })
    }
}

impl ToTokens for State {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        
        let vis = match &self.visibility {
            syn::Visibility::Inherited => quote! { pub },
            v => quote! { #v },
        };
        
        let attrs = &self.attrs;

        if let Some(_braces) = &self.braces {
            let fields = self.fields.iter().flatten().map(|f| {
                let mut field = f.clone();
                if matches!(field.visibility, None | Some(syn::Visibility::Inherited)) {
                    field.visibility = Some(syn::parse_quote! { pub });
                }
                field
            });

            tokens.extend(quote! {
                #(#attrs)*
                #vis struct #name {
                    #(#fields),*
                }
            });
        } else if let Some(_parens) = &self.parens {
            let tuple_fields = self.tuple_fields.iter().flatten().map(|f| {
                let mut field = f.clone();
                if matches!(field.vis, syn::Visibility::Inherited) {
                    field.vis = syn::parse_quote! { pub };
                }
                field
            });

            tokens.extend(quote! {
                #(#attrs)*
                #vis struct #name(#(#tuple_fields),*);
            });
        } else {
            tokens.extend(quote! {
                #(#attrs)*
                #vis struct #name;
            });
        }
    }
}