#[macro_export]
macro_rules! specialized_ser_via_display_inner {
    ($ty: ty, $trait: ident) => {
        impl<S: Serializer> $trait<S> for $ty {
            fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
                serializer.serialize_str(&self.to_string())
            }
        }
    };
}

#[macro_export]
macro_rules! specialized_ser_via_display {
    ($ty: ty) => {
        $crate::specialized_ser_via_display_inner!($ty, $crate::SpecializedSerialize);
    };
}
