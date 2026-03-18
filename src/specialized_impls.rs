//! Specialized [`Ser`] implementations for standard library types.
//!
//! These implementations handle types that cannot be serialized through the
//! generic reflection-based blanket impl, either because they are unsized
//! (e.g. `[T]`, `str`) or because they require custom serialization logic
//! (e.g. `Option<T>`, `String`, `Vec<T>`, smart pointers, collections).
//!
//! The implementations are split into two categories:
//!
//! - **Direct `Ser` impls**: for unsized types (`[T]`, `str`, `OsStr`, `Path`)
//!   that cannot go through the `T: 'static` blanket impl.
//! - **`SpecializedSerInner` impls**: for sized types (`Option<T>`, `&str`,
//!   `String`, `Vec<T>`, `Box<T>`, collections, etc.) that override the blanket
//!   impl via `try_as_dyn` dispatch.

use super::*;
use std::{ffi::OsStr, ops::Deref, path::Path};

/// Serializes a slice `[T]` as a sequence.
impl<T: Ser<S>, S: Serializer + 'static> Ser<S> for [T] {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for elem in self {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}

/// Serializes a `str` slice as a string.
impl<S: Serializer> Ser<S> for str {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

/// Serializes an `OsStr` as a string (with lossy UTF-8 conversion).
impl<S: Serializer> Ser<S> for OsStr {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string_lossy().as_ref())
    }
}

/// Serializes a `Path` as a string (with lossy UTF-8 conversion).
impl<S: Serializer> Ser<S> for Path {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_os_str().to_string_lossy().as_ref())
    }
}

/// Serializes `Option<T>` as `Some(value)` or `None`.
impl<T: Ser<S>, S: Serializer> SpecializedSerInner<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

/// Serializes `&str` as a string (avoids double-reference indirection).
impl<S: Serializer> SpecializedSerInner<S> for &str {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

// --- Primitive serialization via SpecializedSerInner (feature-gated) ---

#[cfg(feature = "primitive-specialized")]
macro_rules! specialized_ser_primitive {
    ($ty:ty, $method:ident) => {
        impl<S: Serializer + 'static> SpecializedSerInner<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                serializer.$method(*self)
            }
        }
    };
}

#[cfg(feature = "primitive-specialized")]
mod _primitive_impls {
    use super::*;
    specialized_ser_primitive!(bool, serialize_bool);
    specialized_ser_primitive!(char, serialize_char);
    specialized_ser_primitive!(i8, serialize_i8);
    specialized_ser_primitive!(i16, serialize_i16);
    specialized_ser_primitive!(i32, serialize_i32);
    specialized_ser_primitive!(i64, serialize_i64);
    specialized_ser_primitive!(i128, serialize_i128);
    specialized_ser_primitive!(u8, serialize_u8);
    specialized_ser_primitive!(u16, serialize_u16);
    specialized_ser_primitive!(u32, serialize_u32);
    specialized_ser_primitive!(u64, serialize_u64);
    specialized_ser_primitive!(u128, serialize_u128);
    specialized_ser_primitive!(f32, serialize_f32);
    specialized_ser_primitive!(f64, serialize_f64);
}

// --- Display-based serialization (serialized as strings) ---

specialized_ser_via_display_inner!(std::net::IpAddr);
specialized_ser_via_display_inner!(std::net::Ipv4Addr);
specialized_ser_via_display_inner!(std::net::Ipv6Addr);

// --- Deref-based serialization (delegate to the inner type) ---

specialized_ser_via_deref_inner!(String);
specialized_ser_via_deref_inner!(std::path::PathBuf);
specialized_ser_via_deref_inner!(std::ffi::OsString);
specialized_ser_via_deref_inner!(std::borrow::Cow<'_, T>, T: ToOwned + ?Sized);
specialized_ser_via_deref_inner!(std::vec::Vec<T>, T);
specialized_ser_via_deref_inner!(std::boxed::Box<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::rc::Rc<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::sync::Arc<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::cell::RefCell<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::cell::RefMut<'_, T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::cell::Ref<'_, T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::sync::Mutex<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::sync::RwLock<T>, T: ?Sized);
specialized_ser_via_deref_inner!(std::pin::Pin<T>, T);
specialized_ser_via_deref_inner!(std::mem::ManuallyDrop<T>, T: ?Sized);

// --- Sequence-based serialization (serialized as JSON arrays) ---

specialized_ser_seq_inner!(std::collections::VecDeque<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::HashSet<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::BTreeSet<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::BinaryHeap<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::LinkedList<T>, T: Ser<S>);

// --- NonZero serialization (serialized as the underlying integer) ---

macro_rules! specialized_ser_nonzero {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<S: Serializer + 'static> SpecializedSerInner<S> for $ty {
                fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                    self.get().serialize(serializer)
                }
            }
        )*
    };
}

specialized_ser_nonzero!(
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroU128,
    std::num::NonZeroUsize,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroI128,
    std::num::NonZeroIsize,
);

// --- Map-based serialization (serialized as JSON objects) ---

specialized_ser_map_inner!(std::collections::HashMap<K, V>, K: Ser<S>, V: Ser<S>);
specialized_ser_map_inner!(std::collections::BTreeMap<K, V>, K: Ser<S>, V: Ser<S>);
