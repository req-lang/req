use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, GenericArgument, parse_macro_input};

/// Extracts the generic arguments from the `variant` field's type.
///
/// For example, if the struct has:
///   `pub variant: node::entity::Variant<NodeRef<Node>, R, E, M>`
///
/// This returns `[NodeRef<Node>, R, E, M]`, which the derive macros use
/// to build the trait impl without any helper attributes.
fn variant_args(input: &DeriveInput) -> Vec<GenericArgument> {
    let fields = match &input.data {
        syn::Data::Struct(s) => &s.fields,
        _ => panic!("Only structs are supported"),
    };

    let field = fields
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "variant"))
        .expect("Struct must have a `variant` field");

    let path = match &field.ty {
        syn::Type::Path(tp) => &tp.path,
        _ => panic!("`variant` field must be a path type, not a type alias"),
    };

    let last = path.segments.last().expect("Empty path");

    match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => args.args.iter().cloned().collect(),
        _ => panic!(
            "`variant` field type must carry generic arguments — \
             use the full type (e.g. `node::entity::Variant<N, R, E, M>`) \
             instead of a type alias"
        ),
    }
}

/// Derives `ChildrenIter<N>` where `N` is the first generic arg of the `variant` field type.
///
/// ```ignore
/// #[derive(ChildrenIter)]
/// pub struct Node {
///     pub variant: node::entity::Variant<NodeRef<Node>, R, E, M>,
/// }
/// ```
#[proc_macro_derive(ChildrenIter)]
pub fn derive_children_iter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let args = variant_args(&input);
    let n = &args[0];

    quote! {
        impl ChildrenIter<#n> for #struct_name {
            fn children_iter(&self) -> impl Iterator<Item = #n> {
                self.variant
                    .children_iter()
                    .map(|n| n.clone())
                    .collect::<Vec<_>>()
                    .into_iter()
            }
        }
    }
    .into()
}

/// Derives `node::entity::IntoVariable<N, R, E, M>` where `N, R, E, M` are the
/// four generic args of the `variant` field type.
///
/// ```ignore
/// #[derive(IntoEntityVariable)]
/// pub struct Node {
///     pub variant: node::entity::Variant<NodeRef<Node>, R, NodeRef<expression::Node>, markup::Node>,
/// }
/// ```
#[proc_macro_derive(IntoEntityVariable)]
pub fn derive_into_entity_variable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let args = variant_args(&input);
    assert_eq!(args.len(), 4, "entity Variant expects 4 type args: N, R, E, M");
    let (n, r, e, m) = (&args[0], &args[1], &args[2], &args[3]);

    quote! {
        impl IntoVariable<#n, #r, #e, #m> for #struct_name {
            fn into_variant(self) -> crate::node::entity::Variant<#n, #r, #e, #m> {
                self.variant
            }
        }
    }
    .into()
}

/// Derives `node::expression::IntoVariable<N, R>` where `N, R` are the two
/// generic args of the `variant` field type.
///
/// ```ignore
/// #[derive(IntoExpressionVariable)]
/// pub struct Node {
///     pub variant: node::expression::Variant<NodeRef<Node>, R>,
/// }
/// ```
#[proc_macro_derive(IntoExpressionVariable)]
pub fn derive_into_expression_variable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let args = variant_args(&input);
    assert_eq!(args.len(), 2, "expression Variant expects 2 type args: N, R");
    let (n, r) = (&args[0], &args[1]);

    quote! {
        impl IntoVariable<#n, #r> for #struct_name {
            fn into_variant(self) -> crate::node::expression::Variant<#n, #r> {
                self.variant
            }
        }
    }
    .into()
}

/// Derives `node::entity::VariableMut<N, R, E, M>` where `N, R, E, M` are the
/// four generic args of the `variant` field type.
///
/// ```ignore
/// #[derive(EntityVariableMut)]
/// pub struct Node {
///     pub variant: node::entity::Variant<NodeRef<Node>, R, NodeRef<expression::Node>, markup::Node>,
/// }
/// ```
#[proc_macro_derive(EntityVariableMut)]
pub fn derive_entity_variable_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let args = variant_args(&input);
    assert_eq!(args.len(), 4, "entity Variant expects 4 type args: N, R, E, M");
    let (n, r, e, m) = (&args[0], &args[1], &args[2], &args[3]);

    quote! {
        impl VariableMut<#n, #r, #e, #m> for #struct_name {
            fn variant_mut(&mut self) -> &mut crate::node::entity::Variant<#n, #r, #e, #m> {
                &mut self.variant
            }
        }
    }
    .into()
}
