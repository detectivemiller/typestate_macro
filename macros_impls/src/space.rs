use std::{fmt::Display, ops::{Deref, DerefMut}};

use quote::{ToTokens, quote};
use syn::{Attribute, Error, Expr, Ident, Lit, Meta, Token, braced, bracketed, parse::{Parse, ParseStream}, punctuated::Punctuated};

use crate::state::State;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SpaceName(pub Ident);

impl Display for SpaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SpaceAbrreviation(Ident);

impl ToTokens for SpaceAbrreviation {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

macro_rules! deref_impl {
    ($($structure:ty),+) => {
        $(
            impl Deref for $structure {
                type Target = Ident;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl DerefMut for $structure {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        )+
    };
}

deref_impl!(SpaceName, SpaceAbrreviation);

type PrefixSize = usize;

impl SpaceName {
    pub fn create_abbr(&mut self, n: PrefixSize) -> Result<SpaceAbrreviation, syn::Error> {
        // name of a space should be snake_case and also consider some lint for not having the same first and after _ letters?
        // TODO we should check all the previous abbrs for duplication
        let abbr: String = self
            .to_string()
            .split('_')
            .map(|sub| {
                let chars: Vec<char> = sub.chars().collect();

                if chars.len() < n {
                    return Err(Error::new(self.span(), "specified prefix too long for this name"));
                }
                
                Ok(sub.chars().take(n).collect::<String>())
            }).collect::<Result<String, Error>>()?; // we collect to Result but because `create_abbr` returns `Result<Ident, syn::Error>` we can propagate the error using `?` leave only the string

            Ok(SpaceAbrreviation(Ident::new(&abbr, self.span())))
    }
}

impl Parse for SpaceName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        let name = ident.to_string();

        if name.is_empty() || name.starts_with('_') || name.ends_with('_') {
            return Err(syn::Error::new(ident.span(), "a name should be provided"));
        }

        let is_valid = name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
        });

        if !is_valid {
            return Err(syn::Error::new(ident.span(), "the name should be snake_case with ascii characters"));
        }

        Ok(SpaceName(ident))
    }
}

impl ToTokens for SpaceName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}
// Space containing its possible states
pub struct Space {
    pub attributes: Vec<Attribute>,
    pub name: SpaceName,
    pub states: Punctuated<State, Token![,]>,
    pub default: Option<Expr>, // initializing with some values
    pub abbreviation: SpaceAbrreviation
}

impl Parse for Space {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // parse attrs first
        let attributes = input.call(Attribute::parse_outer)?;

        // parse the ident (that will become the SpaceName)
        let mut name = input.parse::<SpaceName>()?;

        let mut abbr: Option<SpaceAbrreviation> = None;
        // check for a user provided abbr, using #[abbr = "value"]
        // add explicit error checking
        for attr in &attributes {
            if attr.path().is_ident("abbr")
                && let Meta::NameValue(meta) = &attr.meta
                    && let Expr::Lit(literal) = &meta.value
                        && let Lit::Str(lit) = &literal.lit {
                            println!("inside the user provided check");
                            abbr = Some(SpaceAbrreviation(Ident::new(&lit.value(), lit.span())));
                        }
        }

        println!("abbr variable is: {:?}", abbr);

        let prefix_size: PrefixSize = 2;
        let abbreviation = match abbr {
            Some(user_typed) => user_typed,
            None => name.create_abbr(prefix_size)?, // 2 should be the default, but we provide via another macro
        };

        input.parse::<Token![=]>()?;

        // unpack braces
        let braced_content;
        braced!(braced_content in input);

        // unpack brackets
        let bracket_content;
        bracketed!(bracket_content in braced_content);
        let states = bracket_content.parse_terminated(State::parse, Token![,])?;

        // parsing the optional default
        let mut default = None;
        if !braced_content.is_empty() {
            braced_content.parse::<Token![,]>()?;
            let word = braced_content.parse::<Ident>()?;
            if word.to_string().to_lowercase() == "default" {
                braced_content.parse::<Token![=]>()?;
                default = Some(braced_content.parse::<Expr>()?);
            } else {
                return Err(syn::Error::new(
                    word.span(),
                    format!("unknown `{word}`, a default can be specified here by `default = `"),
                ));
            }
        }

        Ok(Self {
            attributes,
            name,
            states,
            default,
            abbreviation,
        })
    }
}
