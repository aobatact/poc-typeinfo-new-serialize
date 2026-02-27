use super::*;
use std::{ffi::OsStr, ops::Deref, path::Path};

impl<T: 'static, S: Serializer + 'static> Ser<S> for [T] {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for elem in self {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}

impl<S: Serializer> Ser<S> for str {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

impl<S: Serializer> Ser<S> for OsStr {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string_lossy().as_ref())
    }
}

impl<S: Serializer> Ser<S> for Path {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_os_str().to_string_lossy().as_ref())
    }
}

impl<T: Ser<S>, S: Serializer> SpecializedSerInner<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

impl<S: Serializer> SpecializedSerInner<S> for &str {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

specialized_ser_via_display_inner!(std::net::IpAddr);
specialized_ser_via_display_inner!(std::net::Ipv4Addr);
specialized_ser_via_display_inner!(std::net::Ipv6Addr);

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

specialized_ser_seq_inner!(std::collections::VecDeque<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::HashSet<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::BTreeSet<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::BinaryHeap<T>, T: Ser<S>);
specialized_ser_seq_inner!(std::collections::LinkedList<T>, T: Ser<S>);

specialized_ser_map_inner!(std::collections::HashMap<K, V>, K: Ser<S>, V: Ser<S>);
specialized_ser_map_inner!(std::collections::BTreeMap<K, V>, K: Ser<S>, V: Ser<S>);
