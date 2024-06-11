use serde::{de::DeserializeOwned, Serialize};


pub trait SerializableState {
    fn to_bytes(&self) -> Result<Vec<u8>, String>;
}

pub trait DeserializableState: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String>;
}

impl<S: Serialize> SerializableState for S {
    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(self).map_err(|e| e.to_string())
    }
}

impl<S: DeserializeOwned> DeserializableState for S {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
    }
}