pub use paste::paste;

#[cfg(feature = "hot_internal")]
mod hot {
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
}

#[macro_export]
macro_rules! reloadable_app {
    () => {};
    ($serializable:ident, $shared:ident, $f:ident ($param:ident) $body:block) => {
        xilem_dexterous_developer::macros::paste!(reloadable_app!(@inner $f, $param, $serializable, $shared, [<$f _mod>], [<$f _raw>], [<$f _dexterous_developered_inner>], $body););
    };
    (@inner $f:ident, $param:ident, $serializable:ident, $shared:ident, $mod_f: tt, $raw_f: tt, $inner_f:tt, $body:block) => {
        mod $mod_f {
            use super::*;

            type _SharedStateType = $shared;
            type _SerializableStateType = $serializable;

            fn $raw_f($param: &mut xilem_dexterous_developer::InternalReloadableState<$shared>) -> impl xilem::WidgetView<xilem_dexterous_developer::InternalReloadableState<$shared>> {
                $body
            }


            #[no_mangle]
            pub fn $inner_f(state: &mut xilem_dexterous_developer::InternalReloadableState<$shared>) -> Box<xilem::AnyWidgetView<xilem_dexterous_developer::InternalReloadableState<$shared>>> {
                Box::new($raw_f(state))
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, Debug)]
        pub struct $f;

        impl xilem_dexterous_developer::ReloadableAppLogic<$shared> for $f {
            fn function_name() -> &'static str {
                stringify!($inner_f)
            }

            fn call_default(&self, state: &mut xilem_dexterous_developer::InternalReloadableState<$shared>) -> Box<xilem::AnyWidgetView<xilem_dexterous_developer::InternalReloadableState<$shared>>>{
                $mod_f::$inner_f(state)
            }
        }
    };
}

#[macro_export]
macro_rules! state {
    () => {
        &mut InternalReloadableState<_SharedStateType>        
    };
}

#[macro_export]
macro_rules! interpret {
    ($state:ident) => {
        $state.interpret::<_SerializableStateType>()
    };
}
