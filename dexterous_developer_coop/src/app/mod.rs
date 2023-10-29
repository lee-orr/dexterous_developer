pub trait CoopedApp {
    fn build(&mut self);

    fn run_frame(&mut self);

    fn send_event<T: rkyv::Serialize<S> + rkyv::Deserialize<T, S>, S: rkyv::Fallible>(&mut self);

    fn register_shared_blackboard<
        T: rkyv::Serialize<S> + rkyv::Deserialize<T, S>,
        S: rkyv::Fallible,
    >(
        &mut self,
    );
}
