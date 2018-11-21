/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![recursion_limit = "128"]

#[macro_use]
extern crate darling;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;
extern crate synstructure;

use proc_macro::TokenStream;

mod cg;
mod keyword;
mod property_declaration;
mod break_token;

#[proc_macro_derive(Keyword, attributes(css, parse))]
pub fn derive_parse(stream: TokenStream) -> TokenStream {
    let input = syn::parse(stream).unwrap();
    keyword::derive(input).into()
}

#[proc_macro_derive(PropertyDeclaration, attributes(declaration))]
pub fn derive_property_declaration(stream: TokenStream) -> TokenStream {
    let input = syn::parse(stream).unwrap();
    property_declaration::derive(input).into()
}

#[proc_macro_derive(BreakToken)]
pub fn derive_break_token(stream: TokenStream) -> TokenStream {
    let input = syn::parse(stream).unwrap();
    break_token::derive(input).into()
}
