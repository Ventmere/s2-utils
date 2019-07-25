use proc_macro::TokenStream;
use proc_macro2::Span;

use syn::punctuated::Pair;
use syn::*;

struct VariantConfig<'a> {
    name: &'a str,
    name_span: Span,
    rename: String,
    is_unknown: bool,
    is_tuple: bool,
}

pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(input).unwrap();

    let enum_name = ast.ident;
    let enabled_impls = parse_enabled_impls(&ast.attrs);
    let variants = parse_variants(&ast.data);

    if variants.iter().all(|v| !v.is_unknown) {
        panic!("#[derive(StrEnum)] variant for unknown values not defined");
    }

    let from_str_items: Vec<_> = variants
        .iter()
        .map(
            |&VariantConfig {
                 name,
                 name_span,
                 ref rename,
                 is_unknown,
                 is_tuple,
             }| {
                let name_tokens = Ident::new(name, name_span);
                if is_unknown {
                    if is_tuple {
                        quote! {
                          v => #enum_name::#name_tokens(v.to_owned()),
                        }
                    } else {
                        quote! {
                          _ => #enum_name::#name_tokens,
                        }
                    }
                } else {
                    quote! {
                      #rename => #enum_name::#name_tokens,
                    }
                }
            },
        )
        .collect();

    let to_str_items: Vec<_> = variants
        .iter()
        .map(
            |&VariantConfig {
                 name,
                 name_span,
                 ref rename,
                 is_unknown,
                 is_tuple,
             }| {
                let name_tokens = Ident::new(name, name_span);
                if is_unknown {
                    if is_tuple {
                        quote! {
                          #enum_name::#name_tokens(_) => #rename,
                        }
                    } else {
                        quote! {
                          #enum_name::#name_tokens => #rename,
                        }
                    }
                } else {
                    quote! {
                      #enum_name::#name_tokens => #rename,
                    }
                }
            },
        )
        .collect();

    let deref_str_items: Vec<_> = variants
        .iter()
        .map(
            |&VariantConfig {
                 name,
                 name_span,
                 ref rename,
                 is_unknown,
                 is_tuple,
             }| {
                let name_tokens = Ident::new(name, name_span);
                if is_unknown {
                    if is_tuple {
                        quote! {
                          #enum_name::#name_tokens(ref v) => v,
                        }
                    } else {
                        quote! {
                          #enum_name::#name_tokens => #rename,
                        }
                    }
                } else {
                    quote! {
                      #enum_name::#name_tokens => #rename,
                    }
                }
            },
        )
        .collect();

    let known_variants_items: Vec<_> = variants
        .iter()
        .filter_map(
            |&VariantConfig {
                 name,
                 name_span,
                 is_unknown,
                 ..
             }| {
                let name_tokens = Ident::new(name, name_span);
                if is_unknown {
                    None
                } else {
                    Some(quote! {
                      #enum_name::#name_tokens
                    })
                }
            },
        )
        .collect();

    let diesel_code = if enabled_impls.iter().any(|k| k == "diesel") {
        let as_expr = quote! {
          impl ::diesel::expression::AsExpression<::diesel::sql_types::Text> for #enum_name {
            type Expression = <&'static str as ::diesel::expression::AsExpression<::diesel::sql_types::Text>>::Expression;

            fn as_expression(self) -> Self::Expression {
              <&'static str as ::diesel::expression::AsExpression<::diesel::sql_types::Text>>::as_expression(self.to_str())
            }
          }
        };

        let as_expr_ref = quote! {
          impl<'a> ::diesel::expression::AsExpression<::diesel::sql_types::Text> for &'a #enum_name {
            type Expression = <&'static str as ::diesel::expression::AsExpression<::diesel::sql_types::Text>>::Expression;

            fn as_expression(self) -> Self::Expression {
              <&'static str as ::diesel::expression::AsExpression<::diesel::sql_types::Text>>::as_expression(self.to_str())
            }
          }
        };

        let queryable = quote! {
          impl<DB> ::diesel::query_source::Queryable<::diesel::sql_types::Text, DB> for #enum_name
          where DB: ::diesel::backend::Backend<RawValue = [u8]>
          {
            type Row = <String as ::diesel::query_source::Queryable<::diesel::sql_types::Text, DB>>::Row;
            fn build(row: Self::Row) -> Self {
              let v = <String as ::diesel::query_source::Queryable<::diesel::sql_types::Text, DB>>::build(row);
              Self::from(&v)
            }
          }
        };

        let from_sql = quote! {
          impl<DB> ::diesel::deserialize::FromSql<::diesel::sql_types::Text, DB> for #enum_name
          where DB: ::diesel::backend::Backend<RawValue = [u8]>
          {
            fn from_sql(bytes: Option<&DB::RawValue>) -> ::diesel::deserialize::Result<Self> {
              let str_value = <String as ::diesel::deserialize::FromSql<::diesel::sql_types::Text, DB>>::from_sql(bytes)?;
              Ok(Self::from(&str_value))
            }
          }
        };

        let to_sql = quote! {
          impl<DB> ::diesel::serialize::ToSql<::diesel::sql_types::Text, DB> for #enum_name
          where DB: ::diesel::backend::Backend<RawValue = [u8]>
          {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut ::diesel::serialize::Output<W, DB>) -> ::diesel::serialize::Result {
              <str as ::diesel::serialize::ToSql<::diesel::sql_types::Text, DB>>::to_sql(self.to_str(), out)
            }
          }
        };

        Some(quote! {
          #as_expr
          #as_expr_ref
          #queryable
          #from_sql
          #to_sql
        })
    } else {
        None
    };

    let code = quote! {
      impl<T: AsRef<str>> From<T> for #enum_name {
        fn from(v: T) -> #enum_name {
          match v.as_ref() {
            #(#from_str_items)*
          }
        }
      }

      #[allow(dead_code)]
      impl #enum_name {
        pub fn to_str(&self) -> &'static str {
          match *self {
            #(#to_str_items)*
          }
        }

        pub fn known_variants() -> Vec<#enum_name> {
          let mut values = vec![];
          #(
            values.push(#known_variants_items);
          )*
          values
        }
      }

      impl ::std::ops::Deref for #enum_name {
        type Target = str;

        fn deref(&self) -> &str {
          match *self {
            #(#deref_str_items)*
          }
        }
      }

      #diesel_code
    };

    code.into()
}

fn parse_enabled_impls(attrs: &Vec<Attribute>) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| {
            let meta = a
                .interpret_meta()
                .expect("#[str_enum_impl(..)] cannot interpret");
            match meta {
                Meta::List(MetaList {
                    ref ident,
                    ref nested,
                    ..
                }) if ident == "str_enum_impl" => {
                    nested.iter().next().and_then(|meta| match *meta {
                        NestedMeta::Meta(Meta::Word(ref ident)) => Some(ident.to_string()),
                        _ => panic!("#[str_enum_impl(..)] expects a Meta::Word(..)"),
                    })
                }
                _ => None,
            }
        })
        .collect()
}

fn parse_variants(data: &Data) -> Vec<VariantConfig> {
    match *data {
        Data::Enum(DataEnum { ref variants, .. }) => variants.iter().map(parse_variant).collect(),
        _ => panic!("#[derive(StrEnum)] only supports enum"),
    }
}

fn parse_variant(variant: &Variant) -> VariantConfig {
    if variant.discriminant.is_some() {
        panic!("#[derive(StrEnum)] enum discriminant is not supported");
    }

    let variant_name: &str = variant.ident.as_ref();
    let is_unknown = variant_name == "Unknown";
    let rename = variant
        .attrs
        .iter()
        .filter_map(|a| match a.path.segments.first() {
            Some(Pair::End(&PathSegment { ref ident, .. })) if ident == "str_enum" => {
                a.interpret_meta().and_then(|meta| match meta {
                    Meta::NameValue(MetaNameValue {
                        ident,
                        lit: Lit::Str(ref s),
                        ..
                    }) => {
                        if ident == "rename" {
                            Some(s.value())
                        } else {
                            panic!("#[derive(StrEnum)] unknown config name: '{}'", ident);
                        }
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| get_snake_case_name(variant.ident.as_ref()));

    match variant.fields {
        Fields::Unit => VariantConfig {
            name: variant_name,
            name_span: variant.ident.span,
            rename: rename,
            is_unknown: variant.ident == "Unknown",
            is_tuple: false,
        },
        Fields::Unnamed(ref fields) if is_unknown => {
            match fields.unnamed.iter().next() {
                Some(&Field {
                    ty: Type::Path(TypePath { ref path, .. }),
                    ..
                }) => {
                    if path.segments.len() != 1
                        || path.segments.iter().next().map(|s| s.ident.as_ref()) != Some("String")
                    {
                        panic!("#[derive(StrEnum)] Unknown variant should have String as the only field");
                    }
                }
                None => unreachable!(),
                _ => panic!(
                    "#[derive(StrEnum)] Unknown variant should have String as the only field"
                ),
            }
            VariantConfig {
                name: variant_name,
                name_span: variant.ident.span,
                rename: rename,
                is_unknown: true,
                is_tuple: true,
            }
        }
        _ => {
            panic!("#[derive(StrEnum)] Expects a Unit variant or Tuple variant with name 'Unknown'")
        }
    }
}

fn get_snake_case_name(name: &str) -> String {
    name.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if c.is_uppercase() {
                if i == 0 {
                    vec![c.to_lowercase().next().unwrap()].into_iter()
                } else {
                    vec!['_', c.to_lowercase().next().unwrap()].into_iter()
                }
            } else {
                vec![c].into_iter()
            }
        })
        .collect()
}
