use super::*;

impl<T: Ser<S>, S: Serializer> SpecializedSerInner<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

impl<S: Serializer> SpecializedSerInner<S> for &str {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        serializer.serialize_str(self)
    }
}
impl<S: Serializer> SpecializedSerInner<S> for String {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        serializer.serialize_str(self)
    }
}

impl<T: 'static, S: Serializer + 'static> SpecializedSerInner<S> for Vec<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        self.as_slice().serialize(serializer)
    }
}

specialized_ser_via_display_inner!(std::net::IpAddr, SpecializedSerInner);
specialized_ser_via_display_inner!(std::net::Ipv4Addr, SpecializedSerInner);
specialized_ser_via_display_inner!(std::net::Ipv6Addr, SpecializedSerInner);
