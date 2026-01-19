#![doc = include_str!("../../README.md")]
use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Fields, GenericArgument, Index, PathArguments, Type, TypePath,
    parse_macro_input,
};
#[derive(FromDeriveInput)]
#[darling(supports(struct_named, struct_tuple))]
struct InnerGuardedInput {
    ident: syn::Ident,
    data: darling::ast::Data<(), InnerGuardedField>,
}
#[derive(FromField)]
#[darling(forward_attrs(guard))]
struct InnerGuardedField {
    ident: Option<syn::Ident>,
    ty: Type,
    attrs: Vec<syn::Attribute>,
}

/// Retrieve the inner type T in Arc<Mutex<T>>
fn extract_arc_mutex_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(arc_segment) = path.segments.last()
        && arc_segment.ident == "Arc"
        && let PathArguments::AngleBracketed(arc_args) = &arc_segment.arguments
        && let GenericArgument::Type(mutex_ty) = arc_args.args.first()?
        && let Type::Path(TypePath {
            path: mutex_path, ..
        }) = mutex_ty
        && let Some(mutex_segment) = mutex_path.segments.last()
        && mutex_segment.ident == "Mutex"
        && let PathArguments::AngleBracketed(mutex_args) = &mutex_segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = mutex_args.args.first()
    {
        return Some(inner_ty);
    }
    None
}

enum FieldAccessor {
    Named(syn::Ident),
    Unnamed(Index),
}
impl quote::ToTokens for FieldAccessor {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            FieldAccessor::Named(ident) => ident.to_tokens(tokens),
            FieldAccessor::Unnamed(index) => index.to_tokens(tokens),
        }
    }
}
struct SelectedField<'a> {
    accessor: FieldAccessor,
    inner_type: &'a Type,
}
fn select_field<'a>(fields: &'a [InnerGuardedField]) -> Result<SelectedField<'a>, syn::Error> {
    let marked_fields: Vec<(usize, &InnerGuardedField)> = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| has_guard_attr(&f.attrs))
        .collect();
    match marked_fields.len() {
        0 => {
            if fields.len() == 1 {
                let field = &fields[0];
                let inner_type = extract_arc_mutex_inner(&field.ty).ok_or_else(|| {
                    syn::Error::new_spanned(&field.ty, "Field type must be Arc<Mutex<T>>")
                })?;
                let accessor = match &field.ident {
                    Some(ident) => FieldAccessor::Named(ident.clone()),
                    None => FieldAccessor::Unnamed(Index::from(0)),
                };
                Ok(SelectedField {
                    accessor,
                    inner_type,
                })
            } else {
                Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "Multiple fields found. Use #[guard] to mark which field to use.",
                ))
            }
        }
        1 => {
            let (index, field) = marked_fields[0];
            let inner_type = extract_arc_mutex_inner(&field.ty).ok_or_else(|| {
                syn::Error::new_spanned(
                    &field.ty,
                    "Field marked with #[guard] must be Arc<parking_lot::Mutex<T>>",
                )
            })?;
            let accessor = match &field.ident {
                Some(ident) => FieldAccessor::Named(ident.clone()),
                None => FieldAccessor::Unnamed(Index::from(index)),
            };
            Ok(SelectedField {
                accessor,
                inner_type,
            })
        }
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Only one field can be marked with #[guard]",
        )),
    }
}

fn has_guard_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("guard"))
}

#[proc_macro_derive(InnerGuard, attributes(guard))]
pub fn inner_guarded_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if let Data::Struct(ref data_struct) = input.data
        && matches!(data_struct.fields, Fields::Unit)
    {
        panic!("InnerGuard cannot be derived for unit structs");
    }
    let parsed = match InnerGuardedInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let struct_name = &parsed.ident;
    let fields: Vec<InnerGuardedField> = match parsed.data {
        darling::ast::Data::Struct(fields) => fields.fields,
        _ => unreachable!("Only structs are supported"),
    };
    let selected = match select_field(&fields) {
        Ok(s) => s,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    let accessor = &selected.accessor;
    let inner_type = selected.inner_type;
    let expanded = quote! {
        impl guard_inner_mutex::InnerGuarded<#inner_type> for #struct_name {
            fn lock(&self) -> guard_inner_mutex::InnerGuard<'_, #inner_type> {
                guard_inner_mutex::InnerGuard(self.#accessor.lock())
            }
        }
    };
    TokenStream::from(expanded)
}
