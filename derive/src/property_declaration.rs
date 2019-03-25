use cg;
use quote::Tokens;
use syn::{DeriveInput, Ident, Path, Type};
use synstructure;

#[darling(attributes(declaration), default)]
#[derive(Default, FromVariant)]
pub struct DeclarationVariantAttrs {
    pub field: Option<Ident>,
    pub logical: bool,
    pub early: bool,
}

pub fn derive(input: DeriveInput) -> Tokens {
    let name = &input.ident;
    let s = synstructure::Structure::new(&input);

    let is_early_body = s.each_variant(|variant| {
        let variant_attrs =
            cg::parse_variant_attrs_from_ast::<DeclarationVariantAttrs>(&variant.ast());
        let early = variant_attrs.early;
        quote! { #early }
    });

    let compute_body = s.each_variant(|variant| {
        let variant_attrs =
            cg::parse_variant_attrs_from_ast::<DeclarationVariantAttrs>(&variant.ast());

        let bindings = variant.bindings();
        assert_eq!(bindings.len(), 1);
        assert!(!variant_attrs.logical || variant_attrs.field.is_none());

        let property_name = cg::to_css_identifier(variant.ast().ident.as_ref());
        let field_name = property_name.replace("-", "_");
        let value = &bindings[0];
        let value = quote! { #value.clone() };
        if !variant_attrs.logical {
            let field_name = variant_attrs.field.unwrap_or(Ident::from(field_name));
            return quote! { style.#field_name = #value };
        }

        if property_name.contains("block-size") || property_name.contains("inline-size") {
            let is_block = property_name.contains("block-size");
            let pattern_to_replace = if is_block {
                "block_size"
            } else {
                "inline_size"
            };

            let width = Ident::from(field_name.replace(pattern_to_replace, "width"));
            let height = Ident::from(field_name.replace(pattern_to_replace, "height"));

            let maybe_neg = if is_block {
                quote! {}
            } else {
                quote! { ! }
            };
            quote! {
                if #maybe_neg style.writing_mode.is_vertical() {
                    style.#height= #value;
                } else {
                    style.#width = #value;
                }
            }
        } else {
            assert!(
                property_name.contains("inline-start") ||
                    property_name.contains("inline-end") ||
                    property_name.contains("block-start") ||
                    property_name.contains("block-end")
            );

            let pattern_to_replace = if property_name.contains("inline-start") {
                "inline_start"
            } else if property_name.contains("inline-end") {
                "inline_end"
            } else if property_name.contains("block-start") {
                "block_start"
            } else {
                "block_end"
            };

            let field_name = field_name.replace("inset_", "");

            let function_name = Ident::from(pattern_to_replace.to_owned() + "_physical_side");

            let top = Ident::from(field_name.replace(pattern_to_replace, "top"));
            let bottom = Ident::from(field_name.replace(pattern_to_replace, "bottom"));
            let left = Ident::from(field_name.replace(pattern_to_replace, "left"));
            let right = Ident::from(field_name.replace(pattern_to_replace, "right"));

            quote! {
                let mut field = match style.writing_mode.#function_name() {
                    ::logical_geometry::PhysicalSide::Left => &mut style.#left,
                    ::logical_geometry::PhysicalSide::Right => &mut style.#right,
                    ::logical_geometry::PhysicalSide::Top => &mut style.#top,
                    ::logical_geometry::PhysicalSide::Bottom => &mut style.#bottom,
                };
                *field = #value;
            }
        }
    });

    fn known_parse_function(path: &Path) -> Option<Ident> {
        Some(Ident::from(
            match path.segments.last().unwrap().value().ident.as_ref() {
                "LengthPercentage" => "parse_length_or_percentage",
                "LengthPercentageOrAuto" => "parse_length_or_percentage_or_auto",
                "Length" => "parse_length",
                "Size" => "parse_size",
                "Percentage" => "parse_percentage",
                "Color" => "parse_color",
                "RGBA" => "parse_rgba",
                _ => return None,
            },
        ))
    }

    let parse_body = s.variants().iter().fold(quote!(), |parse_body, variant| {
        let field = &variant.bindings()[0].ast();
        let ty_path = match field.ty {
            Type::Path(ref ty_path) => ty_path,
            ref other => panic!("Unhandled type {:?}", other),
        };

        let parse = match known_parse_function(&ty_path.path) {
            Some(function) => quote! { #function(input) },
            None => quote! { #ty_path::parse(input) },
        };

        let ident = &variant.ast().ident;
        let property_name = cg::to_css_identifier(ident.as_ref());
        quote! {
            #parse_body
            #property_name => Ok(#name::#ident(#parse?)),
        }
    });

    quote! {
        impl #name {
            #[inline]
            fn parse_longhand<'i, 't>(
                name: &::cssparser::CowRcStr<'i>,
                input: &mut ::cssparser::Parser<'i, 't>,
            ) -> Result<Self, ParseError<'i>> {
                let location = input.current_source_location();
                match &**name {
                    #parse_body
                    _ => {
                        Err(location.new_custom_error(
                            ::css::Error::UnknownPropertyName(name.clone())
                        ))
                    }
                }
            }

            fn compute(&self, style: &mut ::style::MutableComputedStyle) {
                match *self {
                    #compute_body
                }
            }

            fn is_early(&self) -> bool {
                match *self {
                    #is_early_body
                }
            }
        }
    }
}
