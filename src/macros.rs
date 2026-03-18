/// Helper macro that implements a serialization trait for a type by delegating
/// to its [`Display`](std::fmt::Display) implementation (serializes as a string).
///
/// Used by [`specialized_ser_via_display_inner!`] and [`specialized_ser_via_display!`].
#[macro_export]
macro_rules! specialized_ser_via_display_base {
    ($ty: ty, $trait: ident) => {
        impl<S: Serializer> $trait<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_string())
            }
        }
    };
}

/// Implements [`SpecializedSerInner`] for a type by delegating to its `Display` impl.
///
/// Use this for standard library types that should be serialized as strings
/// (e.g. `IpAddr`, `Ipv4Addr`).
#[macro_export]
macro_rules! specialized_ser_via_display_inner {
    ($ty: ty) => {
        $crate::specialized_ser_via_display_base!($ty, SpecializedSerInner);
    };
}

/// Implements [`SpecializedSer`] for a type by delegating to its `Display` impl.
///
/// Use this for downstream types that should be serialized as strings.
#[macro_export]
macro_rules! specialized_ser_via_display {
    ($ty: ty) => {
        $crate::specialized_ser_via_display_base!($ty, SpecializedSer);
    };
}

/// Helper macro that implements a serialization trait for a type by delegating
/// to the [`Deref`](std::ops::Deref) target's `Ser` implementation.
///
/// Supports both non-generic types and types with generic parameters.
#[macro_export]
macro_rules! specialized_ser_via_deref_base {
    ($ty: ty, $trait: ident) => {
        impl<S: Serializer + 'static> $trait<S> for $ty where Self: Deref, <Self as Deref>::Target: Ser<S> {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                self.deref().serialize(serializer)
            }
        }
    };

    ($ty: ty, $trait: ident, $($generics: tt)+) => {
        impl<S: Serializer + 'static, $($generics)+> $trait<S> for $ty where Self: Deref, <Self as Deref>::Target: Ser<S> {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                self.deref().serialize(serializer)
            }
        }
    };
}

/// Implements [`SpecializedSerInner`] for a type by serializing through its `Deref` target.
///
/// This is used for wrapper types like `String` (derefs to `str`), `Vec<T>` (derefs to `[T]`),
/// `Box<T>`, `Arc<T>`, etc.
#[macro_export]
macro_rules! specialized_ser_via_deref_inner {
    ($ty: ty) => {
        specialized_ser_via_deref_base!($ty, SpecializedSerInner);
    };

    ($ty: ty, $($generics: tt)+) => {
        specialized_ser_via_deref_base!($ty, SpecializedSerInner, $($generics)+);
    };
}

// #[macro_export]
// macro_rules! specialized_ser_via_deref {
//     ($ty: ty) => {
//         $crate::specialized_ser_via_deref_inner!($ty, $crate::SpecializedSerialize);
//     };
// }

/// Helper macro that implements a serialization trait for an iterable collection
/// type by serializing it as a sequence.
///
/// The type must implement `IntoIterator` (via `for elem in self`) and provide a `len()` method.
#[macro_export]
macro_rules! specialized_ser_seq_base {
    ($ty: ty, $trait: ident, $($generics: tt)+) => {
        impl<S: Serializer + 'static, $($generics)+> $trait<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                let mut seq = serializer.serialize_seq(Some(self.len()))?;
                for elem in self {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
        }
    };
}

/// Implements [`SpecializedSerInner`] for an iterable collection, serializing it as a sequence.
///
/// Used for `VecDeque`, `HashSet`, `BTreeSet`, `BinaryHeap`, `LinkedList`, etc.
#[macro_export]
macro_rules! specialized_ser_seq_inner {
    ($ty: ty, $($generics: tt)+) => {
        specialized_ser_seq_base!($ty, SpecializedSerInner, $($generics)+);
    };
}

/// Helper macro that implements a serialization trait for a map-like collection
/// by serializing it as key-value pairs.
///
/// The type must implement `IntoIterator` yielding `(K, V)` pairs and provide a `len()` method.
#[macro_export]
macro_rules! specialized_ser_map_base {
    ($ty: ty, $trait: ident, $($generics: tt)+) => {
        impl<S: Serializer + 'static, $($generics)+> $trait<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(self.len()))?;
                for (k, v) in self {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    };
}

/// Implements [`SpecializedSerInner`] for a map-like collection, serializing it as a map.
///
/// Used for `HashMap` and `BTreeMap`.
#[macro_export]
macro_rules! specialized_ser_map_inner {
    ($ty: ty, $($generics: tt)+) => {
        specialized_ser_map_base!($ty, SpecializedSerInner, $($generics)+);
    };
}
