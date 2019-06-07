/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::cg;
use quote::Tokens;
use syn::{DeriveInput, Ident};
use synstructure;

pub fn derive(input: DeriveInput) -> Tokens {
    let name = &input.ident;
    let s = synstructure::Structure::new(&input);

    let mut impls = quote! {};
    for variant in s.variants() {
        let bindings = variant.bindings();
        assert_eq!(bindings.len(), 1);

        let ty = &bindings[0].ast().ty;

        let variant_name = &variant.ast().ident;
        let lowercased = cg::to_css_identifier(variant.ast().ident.as_ref()).replace("-", "_");

        let as_name = Ident::from(format!("as_{}", lowercased));
        let into_name = Ident::from(format!("into_{}", lowercased));
        impls = quote! {
            #impls

            impl From<#ty> for #name {
                fn from(inner: #ty) -> Self {
                    #name::#variant_name(inner)
                }
            }

            impl #name {
                #[inline]
                fn #as_name(&self) -> Option<&#ty> {
                    match *self {
                        #name::#variant_name(ref inner) => Some(inner),
                        _ => None,
                    }
                }

                #[inline]
                fn #into_name(&self) -> Option<#ty> {
                    match *self {
                        #name::#variant_name(inner) => Some(inner),
                        _ => None,
                    }
                }
            }
        };
    }

    impls
}
