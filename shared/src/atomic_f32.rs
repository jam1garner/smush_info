use core::sync::atomic::{AtomicU32, Ordering};
use serde::{Serialize, Serializer, Deserialize, Deserializer};

use core::fmt;

#[repr(transparent)]
pub struct AtomicF32(AtomicU32); 

impl AtomicF32 {
    pub const fn new(val: f32) -> Self {
        union Transmute { val: f32, out: u32}
        Self(AtomicU32::new(unsafe { Transmute { val }.out }))
    }

    pub fn load(&self, order: Ordering) -> f32 {
        unsafe {
            core::mem::transmute(self.0.load(order))
        }
    }

    pub fn store(&self, val: f32, order: Ordering) {
        unsafe {
            self.0.store(core::mem::transmute(val), order)
        }
    }
}

impl fmt::Debug for AtomicF32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <f32 as fmt::Debug>::fmt(&self.load(Ordering::SeqCst), f)
    }
}

impl Serialize for AtomicF32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
    {
        serializer.serialize_f32(self.load(Ordering::SeqCst))
    }
}

impl<'de> Deserialize<'de> for AtomicF32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(AtomicF32::new(f32::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod atomic_f32_tests {
    use super::*;

    #[test]
    fn test_serde_round_trip() {
        let x = AtomicF32::new(3.0);
        let json = serde_json::to_string(&x).unwrap();
        assert_eq!(json, "3.0");
        let y: AtomicF32 = serde_json::from_str(&json).unwrap();
        assert_eq!(x.load(Ordering::SeqCst), y.load(Ordering::SeqCst));
    }

    #[derive(Serialize, Deserialize)]
    struct Test {
        pub val: AtomicF32
    }

    #[test]
    fn test_serde_round_trip_struct() {
        let x = Test{ val: AtomicF32::new(3.0) };
        let json = serde_json::to_string(&x).unwrap();
        assert_eq!(json, "{\"val\":3.0}");
        let y: Test = serde_json::from_str(&json).unwrap();
        assert_eq!(x.val.load(Ordering::SeqCst), y.val.load(Ordering::SeqCst));
    }
}
