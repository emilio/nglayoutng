/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use cg;
use quote::Tokens;
use syn::DeriveInput;
use synstructure;

#[darling(attributes(parse), default)]
#[derive(Default, FromVariant)]
pub struct ParseVariantAttrs {
    pub aliases: Option<String>,
}

pub fn derive(input: DeriveInput) -> Tokens {
    let name = &input.ident;
    let s = synstructure::Structure::new(&input);

    let match_body = s.variants().iter().fold(quote!(), |match_body, variant| {
        let bindings = variant.bindings();
        assert!(
            bindings.is_empty(),
            "Parse is only supported for single-variant enums for now"
        );

        let parse_attrs = cg::parse_variant_attrs_from_ast::<ParseVariantAttrs>(&variant.ast());
        let identifier = cg::to_css_identifier(variant.ast().ident.as_ref());
        let ident = &variant.ast().ident;

        let mut body = quote! {
            #match_body
            #identifier => Ok(#name::#ident),
        };

        let aliases = match parse_attrs.aliases {
            Some(aliases) => aliases,
            None => return body,
        };

        for alias in aliases.split(",") {
            body = quote! {
                #body
                #alias => Ok(#name::#ident),
            };
        }

        body
    });

    quote! {
        impl #name {
            /// Parse this keyword.
            #[inline]
            pub fn parse<'i, 't>(
                input: &mut ::cssparser::Parser<'i, 't>,
            ) -> Result<Self, ::layout_tree::builder::css::ParseError<'i>> {
                let location = input.current_source_location();
                let ident = input.expect_ident()?;
                Self::from_ident(ident.as_ref()).map_err(|()| {
                    location.new_unexpected_token_error(
                        ::cssparser::Token::Ident(ident.clone())
                    )
                })
            }

            /// Parse this keyword from a string slice.
            #[inline]
            pub fn from_ident(ident: &str) -> Result<Self, ()> {
                match_ignore_ascii_case! { ident,
                    #match_body
                    _ => Err(()),
                }
            }
        }
    }
}
