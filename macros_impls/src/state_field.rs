use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, Token, Type, Visibility, parse::{Parse, ParseStream}};
use std::ops::{Deref, DerefMut};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct StateName(Ident);

deref_impl!(StateName);

impl Parse for StateName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;

        Ok(StateName(ident))       
    }
}

impl ToTokens for StateName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

// StateField in a state struct
// Currently not supporting attribute-like macros
#[derive(Clone)]
pub struct StateField {
    pub visibility: Option<Visibility>,
    pub name: Ident,
    pub ty: Type,
}

impl Parse for StateField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the optional visibility keyword
        let visibility = input.parse::<Visibility>().ok();
        let name = input.parse::<Ident>()?;

        // Parse the : in 
        // MyStruct {
        //   my_field: type,
        // }
        input.parse::<Token![:]>()?;
        let ty = input.parse::<Type>()?;

        Ok(Self {
            visibility,
            name,
            ty,
        })
    }
}

impl ToTokens for StateField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let vis = &self.visibility;
        let name = &self.name;
        let ty = &self.ty;

        if let Some(vis) = vis {
            tokens.extend(quote! {
                #vis #name: #ty
            })
        } else {
            tokens.extend(quote! {
                #name: #ty
            });
        }
    }
}