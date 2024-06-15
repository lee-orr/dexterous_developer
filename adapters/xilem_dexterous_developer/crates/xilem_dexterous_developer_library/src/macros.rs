pub use paste::paste;

#[cfg(feature = "hot_internal")]
mod hot {

    #[macro_export]
    macro_rules! reloadable_app {
        () => {};
        ($serializable:ident, $shared:ident, $f:ident ($param:ident) $body:block) => {
            xilem_dexterous_developer::macros::paste!(reloadable_app!(@inner $f, $param, $serializable, $shared, [<$f _dexterous_developered_inner>],[<$f _dexterous_developered_serialize>],[<$f _dexterous_developered_deserialize>], $body););
        };
        (@inner $f:ident, $param:ident, $serializable:ident, $shared:ident, $inner_f:tt, $serialize_f:tt, $deserialize_f:tt, $body:block) => {

            #[no_mangle]
            pub fn $inner_f(state: &mut xilem_dexterous_developer::ReloadableState<$shared, $serializable>) -> Box<xilem::AnyWidgetView<xilem_dexterous_developer::ReloadableState<$shared, $serializable>>> {
                $f::call(state)
            }

            #[no_mangle]
            pub fn $serialize_f(state: &mut xilem_dexterous_developer::ReloadableState<$shared, $serializable>) -> xilem_dexterous_developer::ffi::Vec<u8> {
                let serializable = state.serializable();
                let val = xilem_dexterous_developer::ffi::to_vec(serializable).unwrap();
                let val = xilem_dexterous_developer::ffi::Vec::from(val);
                val
            }


            #[no_mangle]
            pub fn $deserialize_f((values, state): &mut (xilem_dexterous_developer::ffi::Vec<u8>, &mut xilem_dexterous_developer::ReloadableState<$shared, $serializable>)) {
                let value = xilem_dexterous_developer::ffi::from_slice(&values).unwrap();
                state.replace_serializable(value);
            }

            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone, Debug)]
            struct $f;

            impl xilem_dexterous_developer::ReloadableAppLogic for $f {
                type State = xilem_dexterous_developer::ReloadableState<$shared, $serializable>;

                fn function_name() -> &'static str {
                    stringify!($inner_f)
                }


                fn serialization_function_name() -> &'static str {
                    stringify!($serialize_f)
                }

                fn deserialization_function_name() -> &'static str {
                    stringify!($deserialize_f)
                }

                fn call($param: &mut Self::State) -> Box<xilem::AnyWidgetView<Self::State>> {
                    Box::new($body)
                }
            }
        };
    }

    #[macro_export]
    macro_rules! reloadable_main {
        ($f:ident() $body:block) => {
            fn reloadable_main_implementation() $body

            #[no_mangle]
            pub extern "Rust" fn dexterous_developer_internal_main() {
                println!("Setting Up Xilem With Hot Reload");
                reloadable_main_implementation();
            }

            pub fn $f() {
                println!("Setting Up Xilem Without Hot Reload");
                reloadable_main_implementation();
            }
        };
    }
}

#[cfg(not(feature = "hot_internal"))]
mod cold {
    #[macro_export]
    macro_rules! reloadable_main {
        ($f:ident() $body:block) => {
            fn reloadable_main_implementation() $body

            pub fn $f() {
                reloadable_main_implementation();
            }
        };
    }

    #[macro_export]
    macro_rules! reloadable_app {
        () => {};
        ($serializable:ident, $f:ident ($param:ident) $body:block) => {
            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone, Debug)]
            struct $f;

            impl xilem_dexterous_developer::ReloadableAppLogic for $f {
                type State = xilem_dexterous_developer::ReloadableState<$serializable>;

                fn call_default(
                    $param: &mut Self::State,
                ) -> Box<xilem::AnyWidgetView<Self::State>> {
                    Box::new($body)
                }
            }
        };
    }
}
