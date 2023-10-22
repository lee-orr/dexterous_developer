extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, ItemFn, Path};

#[proc_macro_attribute]
#[allow(clippy::needless_return)]
pub fn hot_bevy_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);

    let fn_name: &proc_macro2::Ident = &ast.sig.ident;

    let mut stream: Vec<TokenStream> = vec![];
    #[cfg(feature = "hot_internal")]
    {
        stream.push(quote!{

                #[no_mangle]
                pub extern "system" fn dexterous_developer_internal_main(library_paths: std::ffi::CString, closure: fn() -> ()) {
                    #ast

                    dexterous_developer::bevy_support::build_reloadable_frame(library_paths, closure, #fn_name);
                }
            }.into());
    }
    stream.push(
        quote! {
            pub fn #fn_name() {
                #ast

                let mut app = App::new();

                #fn_name(dexterous_developer::InitialPluginsEmpty::new(&mut app));

                app.run();
            }
        }
        .into(),
    );

    TokenStream::from_iter(stream)
}
