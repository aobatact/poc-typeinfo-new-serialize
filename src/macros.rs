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

#[macro_export]
macro_rules! specialized_ser_via_display_inner {
    ($ty: ty) => {
        $crate::specialized_ser_via_display_base!($ty, SpecializedSerInner);
    };
}

#[macro_export]
macro_rules! specialized_ser_via_display {
    ($ty: ty) => {
        $crate::specialized_ser_via_display_base!($ty, SpecializedSer);
    };
}

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

#[macro_export]
macro_rules! specialized_ser_seq_inner {
    ($ty: ty, $($generics: tt)+) => {
        specialized_ser_seq_base!($ty, SpecializedSerInner, $($generics)+);
    };
}

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

#[macro_export]
macro_rules! specialized_ser_map_inner {
    ($ty: ty, $($generics: tt)+) => {
        specialized_ser_map_base!($ty, SpecializedSerInner, $($generics)+);
    };
}
