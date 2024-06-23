pub use paste::paste;

#[cfg(feature = "hot_internal")]
mod hot {

    #[macro_export]
    macro_rules! reloadable_app {
        () => {};
        ($serializable:ident, $f:ident ($param:ident) $body:block) => {
            xilem_dexterous_developer::macros::paste!(reloadable_app!(@inner $f, $param, $serializable,  [<$f _dexterous_developer_inner>],[<$f _dexterous_developer_serialize>],[<$f _dexterous_developer_deserialize>], $body););
        };
        (@inner $f:ident, $param:ident, $serializable:ident, $inner_f:tt, $serialize_f:tt, $deserialize_f:tt, $body:block) => {
            pub struct GuardedState<'a>(&'a mut $serializable);

            impl<'a> std::ops::Deref for GuardedState<'a> {
                type Target = $serializable;
                
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<'a> std::ops::DerefMut for GuardedState<'a> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl GetReloadableStateValue for ReloadableState
            {
                fn get(&mut self) -> GuardedState {
                    println!("Getting Dyn Trait");
                    let mut dyntrait = self.get_dyn_trait();
                    println!("Got Dyn Trait");
                    let downcast = dyntrait.downcast_as_mut::<$serializable>();
                    println!("Has Downcast -{}", downcast.is_err());
                    let downcast = downcast.unwrap();
                    println!("Done Dyn Trait");
                    GuardedState(downcast)
                }

                fn set(&mut self, value: $serializable) {
                    println!("Setting Dyn Trait");
                    let dyntrait = xilem_dexterous_developer::ffi::DynTrait::from_value(value);
                    self.set_dyn_trait(dyntrait);
                }  
            }

            trait GetReloadableStateValue {
                fn get(&mut self) -> GuardedState;

                fn set(&mut self, value: $serializable);
            }

            #[no_mangle]
            pub fn $inner_f(state: &mut xilem_dexterous_developer::ReloadableState) -> Box<xilem::AnyWidgetView<xilem_dexterous_developer::ReloadableState>> {
                $f::call(state)
            }

            #[no_mangle]
            pub fn $serialize_f(state: &mut xilem_dexterous_developer::ReloadableState) -> xilem_dexterous_developer::ffi::Vec<u8> {
                let serializable = state.get();
                let val = xilem_dexterous_developer::ffi::to_vec(serializable.0).unwrap();
                let val = xilem_dexterous_developer::ffi::Vec::from(val);
                val
            }


            #[no_mangle]
            pub fn $deserialize_f((values, state): &mut (xilem_dexterous_developer::ffi::Vec<u8>, &mut xilem_dexterous_developer::ReloadableState)) {
                let value = xilem_dexterous_developer::ffi::from_slice(&values).unwrap();
                state.set(value);
            }

            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone, Debug)]
            struct $f;

            impl xilem_dexterous_developer::ReloadableAppLogic for $f {
                type Serializable = $serializable;
                type State = xilem_dexterous_developer::ReloadableState;

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
