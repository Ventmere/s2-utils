#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;

extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

mod has_item_key;
mod str_enum;
mod struct_mapper;

#[proc_macro_derive(StructMapper, attributes(map_from))]
pub fn derive_struct_mapper(input: TokenStream) -> TokenStream {
  struct_mapper::derive(input)
}

#[proc_macro_derive(StrEnum, attributes(str_enum_impl))]
pub fn derive_str_enum(input: TokenStream) -> TokenStream {
  str_enum::derive(input)
}

#[proc_macro_derive(HasItemKey, attributes(has_item_key))]
pub fn derive_has_item_key(input: TokenStream) -> TokenStream {
  has_item_key::derive(input)
}
