use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Token, Type, parse_macro_input, punctuated::Punctuated};

fn parse_type_params(input: &DeriveInput, attr_name: &str) -> Vec<Type> {
    let attr = input
        .attrs
        .iter()
        .find(|a| a.path().is_ident(attr_name))
        .unwrap_or_else(|| panic!("#[derive] requires a #[{attr_name}(...)] attribute"));

    attr.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)
        .unwrap_or_else(|_| panic!("Expected comma-separated types in #[{attr_name}(...)]"))
        .into_iter()
        .collect()
}

/// Derives `ChildrenIter<N>` by delegating to `self.variant.children_iter()`.
///
/// ```ignore
/// #[derive(ChildrenIter)]
/// #[children_iter(NodeRef<Node>)]
/// pub struct Node { pub variant: SomeVariant }
/// ```
#[proc_macro_derive(ChildrenIter, attributes(children_iter))]
pub fn derive_children_iter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let types = parse_type_params(&input, "children_iter");
    assert_eq!(types.len(), 1, "#[children_iter(...)] expects exactly 1 type");
    let n = &types[0];

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

/// Derives `node::entity::IntoVariable<N, R, E, M>` by returning `self.variant`.
///
/// ```ignore
/// #[derive(IntoEntityVariable)]
/// #[into_entity_variable(NodeRef<Node>, SomeR, NodeRef<expression::Node>, markup::Node)]
/// pub struct Node { pub variant: SomeVariant }
/// ```
#[proc_macro_derive(IntoEntityVariable, attributes(into_entity_variable))]
pub fn derive_into_entity_variable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let types = parse_type_params(&input, "into_entity_variable");
    assert_eq!(
        types.len(),
        4,
        "#[into_entity_variable(...)] expects exactly 4 types: N, R, E, M"
    );
    let (n, r, e, m) = (&types[0], &types[1], &types[2], &types[3]);

    quote! {
        impl IntoVariable<#n, #r, #e, #m> for #struct_name {
            fn into_variant(self) -> crate::node::entity::Variant<#n, #r, #e, #m> {
                self.variant
            }
        }
    }
    .into()
}

/// Derives `node::expression::IntoVariable<N, R>` by returning `self.variant`.
///
/// ```ignore
/// #[derive(IntoExpressionVariable)]
/// #[into_expression_variable(NodeRef<Node>, SomeR)]
/// pub struct Node { pub variant: SomeVariant }
/// ```
#[proc_macro_derive(IntoExpressionVariable, attributes(into_expression_variable))]
pub fn derive_into_expression_variable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let types = parse_type_params(&input, "into_expression_variable");
    assert_eq!(
        types.len(),
        2,
        "#[into_expression_variable(...)] expects exactly 2 types: N, R"
    );
    let (n, r) = (&types[0], &types[1]);

    quote! {
        impl IntoVariable<#n, #r> for #struct_name {
            fn into_variant(self) -> crate::node::expression::Variant<#n, #r> {
                self.variant
            }
        }
    }
    .into()
}

/// Derives `node::entity::VariableMut<N, R, E, M>` by returning `&mut self.variant`.
///
/// ```ignore
/// #[derive(EntityVariableMut)]
/// #[entity_variable_mut(NodeRef<Node>, SomeR, NodeRef<expression::Node>, markup::Node)]
/// pub struct Node { pub variant: SomeVariant }
/// ```
#[proc_macro_derive(EntityVariableMut, attributes(entity_variable_mut))]
pub fn derive_entity_variable_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let types = parse_type_params(&input, "entity_variable_mut");
    assert_eq!(
        types.len(),
        4,
        "#[entity_variable_mut(...)] expects exactly 4 types: N, R, E, M"
    );
    let (n, r, e, m) = (&types[0], &types[1], &types[2], &types[3]);

    quote! {
        impl VariableMut<#n, #r, #e, #m> for #struct_name {
            fn variant_mut(&mut self) -> &mut crate::node::entity::Variant<#n, #r, #e, #m> {
                &mut self.variant
            }
        }
    }
    .into()
}

