/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use darling::FromVariant;
use syn::Variant;
use synstructure::VariantAst;

pub fn parse_variant_attrs_from_ast<A>(variant: &VariantAst) -> A
where
    A: FromVariant,
{
    let v = Variant {
        ident: *variant.ident,
        attrs: variant.attrs.to_vec(),
        fields: variant.fields.clone(),
        discriminant: variant.discriminant.clone(),
    };
    parse_variant_attrs(&v)
}

pub fn parse_variant_attrs<A>(variant: &Variant) -> A
where
    A: FromVariant,
{
    match A::from_variant(variant) {
        Ok(attrs) => attrs,
        Err(e) => panic!("failed to parse variant attributes: {}", e),
    }
}

/// Transforms "FooBar" to "foo-bar".
///
/// If the first Camel segment is "Moz", "Webkit", or "Servo", the result string
/// is prepended with "-".
pub fn to_css_identifier(mut camel_case: &str) -> String {
    camel_case = camel_case.trim_right_matches('_');
    let mut first = true;
    let mut result = String::with_capacity(camel_case.len());
    while let Some(segment) = split_camel_segment(&mut camel_case) {
        if first {
            match segment {
                "Moz" | "Webkit" | "Servo" => first = false,
                _ => {},
            }
        }
        if !first {
            result.push_str("-");
        }
        first = false;
        result.push_str(&segment.to_lowercase());
    }
    result
}

/// Given "FooBar", returns "Foo" and sets `camel_case` to "Bar".
fn split_camel_segment<'input>(camel_case: &mut &'input str) -> Option<&'input str> {
    let index = match camel_case.chars().next() {
        None => return None,
        Some(ch) => ch.len_utf8(),
    };
    let end_position = camel_case[index..]
        .find(char::is_uppercase)
        .map_or(camel_case.len(), |pos| index + pos);
    let result = &camel_case[..end_position];
    *camel_case = &camel_case[end_position..];
    Some(result)
}
