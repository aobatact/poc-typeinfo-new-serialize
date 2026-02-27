#![feature(try_as_dyn)]
#![feature(type_info)]
#![feature(ptr_metadata)]

use core::str;
use std::{
    any::TypeId,
    mem::{MaybeUninit, type_info::TypeKind},
    ptr::DynMetadata,
};

pub mod json;
pub(crate) mod macros;
mod specialized_impls;

pub trait Ser<S: Serializer> {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

pub(crate) trait SpecializedSerInner<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

pub trait Serializer: Sized {
    type Ok;
    type Error: SerializeError;
    type SerializeSeq<'a>: SerializeSeq<Serializer = Self>
    where
        Self: 'a;
    type SerializeTuple<'a>: SerializeTuple<Serializer = Self>
    where
        Self: 'a;
    type SerializeTupleStruct<'a>: SerializeTupleStruct<Serializer = Self>
    where
        Self: 'a;
    type SerializeTupleVariant<'a>: SerializeTupleVariant<Serializer = Self>
    where
        Self: 'a;
    type SerializeMap<'a>: SerializeMap<Serializer = Self>
    where
        Self: 'a;
    type SerializeStruct<'a>: SerializeStruct<Serializer = Self>
    where
        Self: 'a;
    type SerializeStructVariant<'a>: SerializeStructVariant<Serializer = Self>
    where
        Self: 'a;

    fn serialize_bool(&mut self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_i8(&mut self, v: i8) -> Result<Self::Ok, Self::Error>;
    fn serialize_i16(&mut self, v: i16) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(&mut self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(&mut self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_i128(&mut self, v: i128) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(&mut self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u16(&mut self, v: u16) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(&mut self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(&mut self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_u128(&mut self, v: u128) -> Result<Self::Ok, Self::Error>;
    fn serialize_f32(&mut self, v: f32) -> Result<Self::Ok, Self::Error>;
    fn serialize_f64(&mut self, v: f64) -> Result<Self::Ok, Self::Error>;
    fn serialize_char(&mut self, v: char) -> Result<Self::Ok, Self::Error>;
    fn serialize_str(&mut self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_bytes(&mut self, v: &[u8]) -> Result<Self::Ok, Self::Error>;
    fn serialize_none(&mut self) -> Result<Self::Ok, Self::Error>;
    fn serialize_some<T: Ser<Self>>(&mut self, value: &T) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit(&mut self) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_struct(&mut self, name: &'static str) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_newtype_struct<T: Ser<Self>>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_newtype_variant<T: Ser<Self>>(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_seq(&mut self, len: Option<usize>) -> Result<Self::SerializeSeq<'_>, Self::Error>;
    fn serialize_tuple(&mut self, len: usize) -> Result<Self::SerializeTuple<'_>, Self::Error>;
    fn serialize_tuple_struct(
        &mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct<'_>, Self::Error>;
    fn serialize_tuple_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant<'_>, Self::Error>;
    fn serialize_map(&mut self, len: Option<usize>) -> Result<Self::SerializeMap<'_>, Self::Error>;
    fn serialize_struct(
        &mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct<'_>, Self::Error>;
    fn serialize_struct_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant<'_>, Self::Error>;
}

pub trait SerializeError {}

pub trait SerializeSeq {
    type Serializer: Serializer;
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeTuple {
    type Serializer: Serializer;
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeTupleStruct {
    type Serializer: Serializer;
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeTupleVariant {
    type Serializer: Serializer;
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeMap {
    type Serializer: Serializer;
    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &V,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn serialize_entry<K: Ser<Self::Serializer> + ?Sized, V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), <Self::Serializer as Serializer>::Error> {
        self.serialize_key(key)?;
        self.serialize_value(value)
    }
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeStruct {
    type Serializer: Serializer;
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

pub trait SerializeStructVariant {
    type Serializer: Serializer;
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

type DynSer<S> = DynMetadata<dyn Ser<S>>;

const MAX_FIELDS: usize = 20;

enum TypeSer<S: 'static> {
    Primitive(TypeKind),
    Tuple {
        fields: [MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS],
        len: usize,
    },
    Struct {
        fields: [MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS],
        len: usize,
    },
    Array {
        len: usize,
        elem: SerTypeInfo<S>,
    },
    // Slice {
    //     elem: SerTypeInfo<S>,
    // },
    Reference {
        referent: SerTypeInfo<S>,
    },
    Other,
}

struct SerFieldInfo<S: 'static> {
    name: &'static str,
    offset: usize,
    vtable: DynSer<S>,
}

struct SerTypeInfo<S: 'static> {
    #[allow(unused)]
    name: &'static str,
    size: usize,
    vtable: DynSer<S>,
}

impl<S: 'static> SerFieldInfo<S> {
    const unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
        unsafe {
            let field_ptr = (ptr as *const T as *const u8).add(self.offset);
            let fat_ptr =
                std::ptr::from_raw_parts::<dyn Ser<S>>(field_ptr as *const (), self.vtable);
            &*fat_ptr
        }
    }
}

impl<S: 'static> SerTypeInfo<S> {
    const unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
        let field_ptr = ptr as *const T as *const u8;
        unsafe {
            let fat_ptr =
                std::ptr::from_raw_parts::<dyn Ser<S>>(field_ptr as *const (), self.vtable);
            &*fat_ptr
        }
    }
}

const fn get_reflect_vtable<S: Serializer + 'static>(type_id: TypeId) -> DynSer<S> {
    let trait_id = TypeId::of::<dyn Ser<S>>();
    match type_id.trait_info_of_trait_type_id(trait_id) {
        Some(t) => unsafe { std::mem::transmute(t.get_vtable()) },
        None => panic!("type does not implement Ser"),
    }
}

impl<S: Serializer + 'static> TypeSer<S> {
    const fn from_type_id(type_id: TypeId) -> Self {
        let type_info = type_id.info();
        match type_info.kind {
            TypeKind::Tuple(tuple_fields) => {
                let mut array = [const { MaybeUninit::<SerFieldInfo<S>>::uninit() }; MAX_FIELDS];
                let mut i = 0;
                while i < tuple_fields.fields.len() && i < MAX_FIELDS {
                    let field = &tuple_fields.fields[i];
                    array[i] = {
                        MaybeUninit::new(SerFieldInfo {
                            name: field.name,
                            offset: field.offset,
                            vtable: get_reflect_vtable::<S>(field.ty),
                        })
                    };
                    i += 1;
                }
                TypeSer::Tuple {
                    fields: array,
                    len: i,
                }
            }
            TypeKind::Struct(struct_fields) => {
                let mut array = [const { MaybeUninit::<SerFieldInfo<S>>::uninit() }; MAX_FIELDS];
                let mut i = 0;
                while i < struct_fields.fields.len() && i < MAX_FIELDS {
                    let field = &struct_fields.fields[i];
                    array[i] = {
                        MaybeUninit::new(SerFieldInfo {
                            name: field.name,
                            offset: field.offset,
                            vtable: get_reflect_vtable::<S>(field.ty),
                        })
                    };
                    i += 1;
                }
                TypeSer::Struct {
                    fields: array,
                    len: i,
                }
            }
            TypeKind::Array(array) => {
                let ty = array.element_ty;
                let type_info = ty.info();
                let elem = SerTypeInfo {
                    name: "", // todo: get type name
                    size: type_info.size.unwrap(),
                    vtable: get_reflect_vtable::<S>(ty),
                };
                TypeSer::Array {
                    len: array.len,
                    elem,
                }
            }
            // TypeKind::Slice(slice) => {
            //     let ty = slice.element_ty;
            //     let type_info = ty.info();
            //     let elem = SerTypeInfo {
            //         name: "", // todo: get type name
            //         size: type_info.size.unwrap(),
            //         vtable: get_reflect_vtable::<S>(ty),
            //     };
            //     TypeSer::Slice { elem }
            // }
            TypeKind::Reference(reference) => {
                let Some(size) = reference.pointee.info().size else {
                    // Unsized types behind references are not supported here, since `try_as_dyn` currently only works for sized types.
                    // When `try_as_dyn` is extended to support `?Sized` types, we can remove this check and handle unsized types behind references as well.
                    return TypeSer::Other;
                };
                let ty = reference.pointee;
                let referent = SerTypeInfo {
                    name: "", // todo: get type name
                    size,
                    vtable: get_reflect_vtable::<S>(ty),
                };
                TypeSer::Reference { referent }
            }
            TypeKind::Bool(_)
            | TypeKind::Char(_)
            | TypeKind::Int(_)
            | TypeKind::Float(_)
            | TypeKind::Str(_) => TypeSer::Primitive(type_info.kind),
            _ => TypeSer::Other,
        }
    }

    pub const fn of<T: 'static + ?Sized>() -> Self {
        const { Self::from_type_id(TypeId::of::<T>()) }
    }
}

impl<T: 'static /* can't add `+ ?Sized` now` */, S: Serializer + 'static> Ser<S> for T {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer)
        } else if let Some(specialized) =
            std::any::try_as_dyn::<_, dyn SpecializedSerInner<S>>(self)
        {
            specialized.specialized_serialize(serializer)
        } else {
            let type_ser = const { TypeSer::<S>::of::<T>() };
            match type_ser {
                TypeSer::Primitive(type_kind) => serialize_primitive(self, serializer, type_kind),
                TypeSer::Struct { fields, len } => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut s = serializer.serialize_struct(std::any::type_name::<T>(), len)?;
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        s.serialize_field(field.name, field_value)?;
                    }
                    s.end()
                },
                TypeSer::Tuple { fields, len } => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut tup = serializer.serialize_tuple(len)?;
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        tup.serialize_element(field_value)?;
                    }
                    tup.end()
                },
                TypeSer::Array { len, elem } => unsafe {
                    let mut seq = serializer.serialize_seq(Some(len))?;
                    for i in 0..len {
                        let field_ptr = (self as *const T as *const u8).add(i * elem.size);
                        let field_value = elem.to_dyn(&*field_ptr.cast::<()>());
                        seq.serialize_element(field_value)?;
                    }
                    seq.end()
                },
                // Slice is handled by the impl for [T] in specialized_impls.rs, so we can assume it's never returned by TypeSer::of
                // When T try_as_dyn can be used for ?Sized types, we can remove this assumption and handle slices here as well.
                // TypeSer::Slice { elem: _ } => unreachable!(),
                TypeSer::Reference { referent } => unsafe {
                    let pointee_ptr = *(self as *const T as *const *const u8);
                    let pointee = referent.to_dyn(&*pointee_ptr.cast::<()>());
                    pointee.serialize(serializer)
                },
                TypeSer::Other => todo!("{} other!", std::any::type_name::<T>()),
            }
        }
    }
}

#[inline]
fn serialize_primitive<T: 'static + ?Sized, S: Serializer>(
    this: &T,
    serializer: &mut S,
    type_kind: TypeKind,
) -> Result<S::Ok, S::Error> {
    match type_kind {
        TypeKind::Bool(_bool) => unsafe {
            let b = *(this as *const T as *const bool);
            serializer.serialize_bool(b)
        },
        TypeKind::Char(_char) => unsafe {
            let c = *(this as *const T as *const char);
            serializer.serialize_char(c)
        },
        TypeKind::Int(int) => {
            if int.signed {
                unsafe {
                    match int.bits {
                        8 => serializer.serialize_i8(*(this as *const T as *const i8)),
                        16 => serializer.serialize_i16(*(this as *const T as *const i16)),
                        32 => serializer.serialize_i32(*(this as *const T as *const i32)),
                        64 => serializer.serialize_i64(*(this as *const T as *const i64)),
                        128 => serializer.serialize_i128(*(this as *const T as *const i128)),
                        _ => unreachable!(),
                    }
                }
            } else {
                unsafe {
                    match int.bits {
                        8 => serializer.serialize_u8(*(this as *const T as *const u8)),
                        16 => serializer.serialize_u16(*(this as *const T as *const u16)),
                        32 => serializer.serialize_u32(*(this as *const T as *const u32)),
                        64 => serializer.serialize_u64(*(this as *const T as *const u64)),
                        128 => serializer.serialize_u128(*(this as *const T as *const u128)),
                        _ => unreachable!(),
                    }
                }
            }
        }
        TypeKind::Float(float) => unsafe {
            match float.bits {
                32 => serializer.serialize_f32(*(this as *const T as *const f32)),
                64 => serializer.serialize_f64(*(this as *const T as *const f64)),
                _ => unreachable!(),
            }
        },
        TypeKind::Str(_str) => {
            unreachable!() // str should be handled by SpecializedSerInner
        }
        _ => unreachable!(),
    }
}
