use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Type, parse_macro_input};

/// Derive macro for `ChildrenIter<N>`.
///
/// # Usage
///
/// The struct must have a `variant` field that already implements `ChildrenIter<N>`.
/// Specify the item type `N` via the `#[children_iter(...)]` helper attribute:
///
/// ```ignore
/// #[derive(ChildrenIter)]
/// #[children_iter(NodeRef<Node>)]
/// pub struct Node {
///     pub variant: SomeVariant,
/// }
/// ```
///
/// This generates:
///
/// ```ignore
/// impl ChildrenIter<NodeRef<Node>> for Node {
///     fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
///         self.variant
///             .children_iter()
///             .map(|n| n.clone())
///             .collect::<Vec<_>>()
///             .into_iter()
///     }
/// }
/// ```
#[proc_macro_derive(ChildrenIter, attributes(children_iter))]
pub fn derive_children_iter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let item_type: Type = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("children_iter"))
        .expect("#[derive(ChildrenIter)] requires a #[children_iter(SomeType)] attribute")
        .parse_args()
        .expect("Expected a type inside #[children_iter(...)]");

    let expanded = quote! {
        impl ChildrenIter<#item_type> for #struct_name {
            fn children_iter(&self) -> impl Iterator<Item = #item_type> {
                self.variant
                    .children_iter()
                    .map(|n| n.clone())
                    .collect::<Vec<_>>()
                    .into_iter()
            }
        }
    };

    expanded.into()
}
