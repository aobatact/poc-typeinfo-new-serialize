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

pub trait Ser<S: Serializer> {
    fn serialize(&self, serializer: &mut S) -> Result<(), S::Error>;
}

pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error>;
}

impl<T: Ser<S>, S: Serializer> SpecializedSer<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

impl<S: Serializer> SpecializedSer<S> for &str {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        serializer.serialize_str(self)
    }
}

pub trait Serializer: Sized {
    type Error;
    type Sequence<'a>: SequenceSerializer<Serializer = Self>
    where
        Self: 'a;
    type Map<'a>: MapSerializer<Serializer = Self>
    where
        Self: 'a;
    type Struct<'a>: StructSerializer<Serializer = Self>
    where
        Self: 'a;

    fn serialize_str(&mut self, value: &str) -> Result<(), Self::Error>;
    fn serialize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn serialize_i16(&mut self, value: i16) -> Result<(), Self::Error>;
    fn serialize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn serialize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    fn serialize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_i128(&mut self, value: i128) -> Result<(), Self::Error>;
    fn serialize_u128(&mut self, value: u128) -> Result<(), Self::Error>;
    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    fn serialize_f32(&mut self, value: f32) -> Result<(), Self::Error>;
    fn serialize_f64(&mut self, value: f64) -> Result<(), Self::Error>;
    fn serialize_unit(&mut self) -> Result<(), Self::Error>;
    fn serialize_some<T: Ser<Self>>(&mut self, value: &T) -> Result<(), Self::Error>;
    fn serialize_none(&mut self) -> Result<(), Self::Error>;
    fn serialize_seq(&mut self) -> Result<Self::Sequence<'_>, Self::Error>;
    fn serialize_map(&mut self) -> Result<Self::Map<'_>, Self::Error>;
    fn serialize_struct(&mut self) -> Result<Self::Struct<'_>, Self::Error>;
}

pub trait SequenceSerializer {
    type Serializer: Serializer;
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(self) -> Result<(), <Self::Serializer as Serializer>::Error>;
}

pub trait MapSerializer {
    type Serializer: Serializer;
    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &V,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(self) -> Result<(), <Self::Serializer as Serializer>::Error>;
}

pub trait StructSerializer {
    type Serializer: Serializer;
    fn serialize_struct_name(
        &mut self,
        struct_name: &str,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn serialize_field_name(
        &mut self,
        field_name: &str,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn serialize_field_value<T: Ser<Self::Serializer>>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    fn end(self) -> Result<(), <Self::Serializer as Serializer>::Error>;
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
    Slice {
        elem: SerTypeInfo<S>,
    },
    Reference {
        mutable: bool,
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
    name: &'static str,
    size: usize,
    vtable: DynSer<S>,
}

impl<S: 'static> SerFieldInfo<S> {
    unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
        unsafe {
            let field_ptr = (ptr as *const T as *const u8).add(self.offset);
            let fat_ptr =
                std::ptr::from_raw_parts::<dyn Ser<S>>(field_ptr as *const (), self.vtable);
            &*fat_ptr
        }
    }
}

impl<S: 'static> SerTypeInfo<S> {
    unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
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
            TypeKind::Slice(slice) => {
                let ty = slice.element_ty;
                let type_info = ty.info();
                let elem = SerTypeInfo {
                    name: "", // todo: get type name
                    size: type_info.size.unwrap(),
                    vtable: get_reflect_vtable::<S>(ty),
                };
                TypeSer::Slice { elem }
            }
            // TypeKind::Reference(reference) => {
            //     let ty = reference.pointee;
            //     let referent = SerFieldInfo {
            //         name: "",
            //         offset: 0,
            //         vtable: get_reflect_vtable::<S>(ty),
            //         type_id: ty,
            //     };
            //     TypeSer::Reference {
            //         mutable: reference.mutable,
            //         referent,
            //     }
            // }
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
    fn serialize(&self, serializer: &mut S) -> Result<(), S::Error> {
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer)
        } else {
            let type_ser = const { TypeSer::<S>::of::<T>() };
            match type_ser {
                TypeSer::Primitive(type_kind) => serialize_primitive(self, serializer, type_kind),
                TypeSer::Struct { fields, len } => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut serializer = serializer.serialize_map()?;
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        serializer.serialize_key(&field.name)?;
                        serializer.serialize_value(field_value)?;
                    }
                    serializer.end()
                },
                TypeSer::Tuple { fields, len } => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut serializer = serializer.serialize_seq()?;
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        serializer.serialize_element(field_value)?;
                    }
                    serializer.end()
                },
                TypeSer::Array { len, elem } => unsafe {
                    let mut serializer = serializer.serialize_seq()?;
                    for i in 0..len {
                        let field_ptr = (self as *const T as *const u8).add(i * elem.size);
                        let field_value = elem.to_dyn(&*field_ptr.cast::<()>());
                        serializer.serialize_element(field_value)?;
                    }
                    serializer.end()
                },
                TypeSer::Slice { elem: _ } => todo!(),
                TypeSer::Reference {
                    mutable: _,
                    referent: _,
                } => todo!(),
                TypeSer::Other => todo!("{} other!", std::any::type_name::<T>()),
            }
        }
    }
}

fn serialize_primitive<T: 'static + ?Sized, S: Serializer>(
    this: &T,
    serializer: &mut S,
    type_kind: TypeKind,
) -> Result<(), S::Error> {
    match type_kind {
        TypeKind::Bool(_bool) => unsafe {
            let b = *(this as *const T as *const bool);
            serializer.serialize_bool(b)
        },
        TypeKind::Char(_char) => unsafe {
            let c = *(this as *const T as *const char);
            let mut buf = [0; 4];
            let s = c.encode_utf8(&mut buf);
            serializer.serialize_str(s)
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
            todo!("xxx")
        }
        _ => unreachable!(),
    }
}
