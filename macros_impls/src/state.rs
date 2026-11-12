use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Visibility, braced, parenthesized};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token,
};

use crate::state_field::StateField;
// Represents a state (which is just a struct) possibly with user provided states
#[derive(Clone)]
pub struct State {
    pub name: Ident,
    pub visibility: Visibility,
    pub attrs: Vec<Attribute>,
    // braces stored for later implementing `ToTokens`
    pub braces: Option<token::Brace>,
    pub fields: Option<Punctuated<StateField, Token![,]>>,
    // In case of a tuple struct
    pub parens: Option<token::Paren>,
    pub tuple_fields: Option<Punctuated<syn::Field, Token![,]>>,
    // Is it shared between spaces (meaning that it can be manipulated via more than one space)
    pub shared: bool,
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
                false
            } else {
                true // Keep other attributes like #[derive(...)]
            }
        });

        let mut braces = None;
        let mut fields = None;
        let mut parens = None;
        let mut tuple_fields = None;

        // If there are braces in the stream => we parse the fields
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
            shared,
        })
    }
}

impl ToTokens for State {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;

        let visibility = match &self.visibility {
            Visibility::Public(_) => quote! { pub },
            Visibility::Restricted(vis_restricted) => quote! { #vis_restricted },
            Visibility::Inherited => quote! {},
        };

        // syn::Visibility::Inherited => quote! { pub },
        // v => quote! { #v },

        let attrs = &self.attrs;

        if let Some(_braces) = &self.braces {
            let fields = self.fields.iter().flatten().map(|f| {
                let field = f.clone();
                // if matches!(field.visibility, None | Some(syn::Visibility::Inherited)) {
                //     field.visibility = Some(syn::parse_quote! { pub });
                // }
                field
            });

            tokens.extend(quote! {
                #(#attrs)*
                #visibility struct #name {
                    #(#fields),*
                }
            });
            // Tuple struct
        } else if let Some(_parens) = &self.parens {
            let tuple_fields = self.tuple_fields.iter().flatten().map(|f| {
                let field = f.clone();
                // if matches!(field.visibility, syn::Visibility::Inherited) {
                //     field.visibility = syn::parse_quote! { pub };
                // }
                field
            });

            tokens.extend(quote! {
                #(#attrs)*
                #visibility struct #name(#(#tuple_fields),*);
            });
            // Unit struct
        } else {
            tokens.extend(quote! {
                #(#attrs)*
                #visibility struct #name;
            });
        }
    }
}
