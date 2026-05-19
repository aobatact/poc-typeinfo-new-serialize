//! Specialized [`SpecializedSerInner`] implementations for standard library types.
//!
//! These implementations handle types that require custom serialization logic
//! (e.g. `Option<T>`, `String`, `Vec<T>`, smart pointers, collections), or
//! unsized types whose default reflection-based serialization is not the
//! desired output (e.g. `OsStr`, `Path`).
//!
//! All specializations are dispatched through `try_as_dyn` in the blanket
//! `Ser` impl, so they work for both sized and unsized types without requiring
//! the `min_specialization` feature.

use super::*;
use std::{ffi::OsStr, ops::Deref, path::Path};

/// Serializes an `OsStr` as a string (with lossy UTF-8 conversion).
impl<S: Serializer> SpecializedSerInner<S> for OsStr {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string_lossy().as_ref())
    }
}

/// Serializes a `Path` as a string (with lossy UTF-8 conversion).
impl<S: Serializer> SpecializedSerInner<S> for Path {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
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
