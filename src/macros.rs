#[macro_export]
macro_rules! specialized_ser_via_display_base {
    ($ty: ty, $trait: ident) => {
        impl<S: Serializer> $trait<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
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
        $crate::specialized_ser_via_display_base!($ty, SpecializedSerialize);
    };
}

#[macro_export]
macro_rules! specialized_ser_via_deref_base {
    ($ty: ty, $trait: ident) => {
        impl<S: Serializer + 'static> $trait<S> for $ty where Self: Deref, <Self as Deref>::Target: Ser<S> {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
                self.deref().serialize(serializer)
            }
        }
    };

    ($ty: ty, $trait: ident, $($generics: tt)+) => {
        impl<S: Serializer + 'static, $($generics)+> $trait<S> for $ty where Self: Deref, <Self as Deref>::Target: Ser<S> {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
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
