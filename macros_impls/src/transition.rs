use syn::Attribute;

use crate::{space::SpaceName, state_field::StateName};

// Arguments for the #[transition()] attribute
pub struct TransitionArgs {
    pub space_name: SpaceName,
    pub from: StateName,
    pub to: StateName,
}

impl TransitionArgs {
    pub fn parse(attr: &Attribute) -> syn::Result<Self> {
        let mut space_name = None;
        let mut from = None;
        let mut to = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("space") {
                space_name = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("from") {
                from = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("to") {
                to = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unsupported transition argument"))
            }
        })?;

        let space_name =
            space_name.ok_or_else(|| syn::Error::new_spanned(attr, "missing `space` argument"))?;
        let from = from.ok_or_else(|| syn::Error::new_spanned(attr, "missing `from` argument"))?;
        let to = to.ok_or_else(|| syn::Error::new_spanned(attr, "missing `to` argument"))?;

        Ok(TransitionArgs {
            space_name,
            from,
            to,
        })
    }
}
