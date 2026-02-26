#![feature(try_as_dyn)]
#![feature(type_info)]
#![feature(ptr_metadata)]

use std::{
    any::TypeId,
    mem::{
        MaybeUninit,
        type_info::{TypeKind},
    },
    ptr::DynMetadata,
};

pub trait Ser<S: Serializer> {
    fn serialize(&self, serializer: &mut S);
}

pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S);
}

impl<T: Ser<S>, S: Serializer> SpecializedSer<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

pub trait Serializer: Sized {
    type Sequence<'a>: SequenceSerializer<Serializer = Self>
    where
        Self: 'a;
    type Map<'a>: MapSerializer<Serializer = Self>
    where
        Self: 'a;

    fn serialize_str(&mut self, value: &str);
    fn serialize_i8(&mut self, value: i8);
    fn serialize_u8(&mut self, value: u8);
    fn serialize_i16(&mut self, value: i16);
    fn serialize_u16(&mut self, value: u16);
    fn serialize_i32(&mut self, value: i32);
    fn serialize_u32(&mut self, value: u32);
    fn serialize_i64(&mut self, value: i64);
    fn serialize_u64(&mut self, value: u64);
    fn serialize_i128(&mut self, value: i128);
    fn serialize_u128(&mut self, value: u128);
    fn serialize_bool(&mut self, value: bool);
    fn serialize_f32(&mut self, value: f32);
    fn serialize_f64(&mut self, value: f64);
    fn serialize_unit(&mut self);
    fn serialize_some<T: Ser<Self>>(&mut self, value: &T);
    fn serialize_none(&mut self);
    fn serialize_seq(&mut self) -> Self::Sequence<'_>;
    fn serialize_map(&mut self) -> Self::Map<'_>;
}

pub trait SequenceSerializer {
    type Serializer: Serializer;
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(&mut self, value: &T);
    fn end(self);
}

pub trait MapSerializer {
    type Serializer: Serializer;
    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(&mut self, key: &K);
    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(&mut self, value: &V);
    fn end(self);
}

type DynSer<S> = DynMetadata<dyn Ser<S>>;

const MAX_FIELDS: usize = 20;

enum TypeSer<S: 'static> {
    Primitive(TypeKind),
    Tuple([MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS], usize),
    Struct([MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS], usize),
    Array {
        len: usize,
        elem: SerFieldInfo<S>,
    },
    Slice {
        elem: SerFieldInfo<S>,
    },
    Reference {
        mutable: bool,
        referent: SerFieldInfo<S>,
    },
    Other,
}

struct SerFieldInfo<S: 'static> {
    name: &'static str,
    offset: usize,
    vtable: DynSer<S>,
    type_id: TypeId,
}

impl<S: 'static> SerFieldInfo<S> {
    unsafe fn to_dyn<T>(&self, ptr: &T) -> &dyn Ser<S> {
        unsafe {
            let field_ptr = (ptr as *const T as *const u8).add(self.offset);
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
                            type_id: field.ty,
                        })
                    };
                    i += 1;
                }
                TypeSer::Tuple(array, i)
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
                            type_id: field.ty,
                        })
                    };
                    i += 1;
                }
                TypeSer::Struct(array, i)
            }
            TypeKind::Array(array) => {
                let ty = array.element_ty;
                let type_info = ty.info();
                let elem = SerFieldInfo {
                    name: "",
                    offset: type_info.size.unwrap(), // for array, the offset of the element is just the size of the element type
                    vtable: get_reflect_vtable::<S>(ty),
                    type_id: ty,
                };
                TypeSer::Array {
                    len: array.len,
                    elem,
                }
            }
            TypeKind::Slice(slice) => {
                let ty = slice.element_ty;
                let type_info = ty.info();
                let elem = SerFieldInfo {
                    name: "",
                    offset: type_info.size.unwrap(), // for slice, the offset of the element is just the size of the element type
                    vtable: get_reflect_vtable::<S>(ty),
                    type_id: ty,
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

    pub const fn of<T: 'static>() -> Self {
        const { Self::from_type_id(TypeId::of::<T>()) }
    }
}

impl<T: 'static, S: Serializer + 'static> Ser<S> for T {
    fn serialize(&self, serializer: &mut S) {
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer);
        } else {
            let type_ser = const { TypeSer::<S>::of::<T>() };
            match type_ser {
                TypeSer::Primitive(type_kind) => {
                    serialize_primitive(self, serializer, type_kind);
                }
                TypeSer::Struct(fields, len) => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut serializer = serializer.serialize_map();
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        serializer.serialize_key(&field.name);
                        serializer.serialize_value(field_value);
                    }
                    serializer.end();
                },
                TypeSer::Tuple(fields, len) => unsafe {
                    let fields = fields[..len].assume_init_ref();
                    let mut serializer = serializer.serialize_seq();
                    for field in fields {
                        let field_value = field.to_dyn(self);
                        serializer.serialize_element(field_value);
                    }
                    serializer.end();
                },
                TypeSer::Array { len, elem } => unsafe {
                    let mut serializer = serializer.serialize_seq();
                    for i in 0..len {
                        let field_ptr = (self as *const T as *const u8).add(i * elem.offset);
                        let field_value = elem.to_dyn(&*field_ptr.cast::<()>());
                        serializer.serialize_element(field_value);
                    }
                    serializer.end();
                },
                TypeSer::Slice { elem: _ } => todo!(),
                TypeSer::Reference { mutable: _, referent: _ } => todo!(),
                TypeSer::Other => todo!(),
            }
        }
    }
}

fn serialize_primitive<T: 'static, S: Serializer>(
    this: &T,
    serializer: &mut S,
    type_kind: TypeKind,
) {
    match type_kind {
        TypeKind::Bool(_bool) => unsafe {
            let b = *(this as *const T as *const bool);
            serializer.serialize_bool(b);
        },
        TypeKind::Char(_char) => unsafe {
            let c = *(this as *const T as *const char);
            let mut buf = [0; 4];
            let s = c.encode_utf8(&mut buf);
            serializer.serialize_str(s);
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
            todo!()
        }
        _ => unreachable!(),
    }
}

pub struct JsonSerializer {
    output: String,
}

impl Serializer for JsonSerializer {
    type Sequence<'b> = JsonSequenceSerializer<'b>;
    type Map<'b> = JsonMapSerializer<'b>;

    fn serialize_str(&mut self, value: &str) {
        self.output.push('"');
        self.output.push_str(value);
        self.output.push('"');
    }

    fn serialize_i8(&mut self, value: i8) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_u8(&mut self, value: u8) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_i16(&mut self, value: i16) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_u16(&mut self, value: u16) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_i32(&mut self, value: i32) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_u32(&mut self, value: u32) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_i64(&mut self, value: i64) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_u64(&mut self, value: u64) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_i128(&mut self, value: i128) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_u128(&mut self, value: u128) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_bool(&mut self, value: bool) {
        self.output.push_str(if value { "true" } else { "false" });
    }

    fn serialize_f32(&mut self, value: f32) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_f64(&mut self, value: f64) {
        self.output.push_str(&value.to_string());
    }

    fn serialize_unit(&mut self) {
        self.output.push_str("null");
    }

    fn serialize_some<T: Ser<Self>>(&mut self, value: &T) {
        value.serialize(self);
    }

    fn serialize_none(&mut self) {
        self.output.push_str("null");
    }

    fn serialize_seq(&mut self) -> Self::Sequence<'_> {
        self.output.push('[');
        JsonSequenceSerializer {
            serializer: self,
            first: true,
        }
    }

    fn serialize_map(&mut self) -> Self::Map<'_> {
        self.output.push('{');
        JsonMapSerializer {
            serializer: self,
            first: true,
        }
    }
}

pub struct JsonSequenceSerializer<'a> {
    serializer: &'a mut JsonSerializer,
    first: bool,
}

impl SequenceSerializer for JsonSequenceSerializer<'_> {
    type Serializer = JsonSerializer;

    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(&mut self, value: &T) {
        if self.first {
            self.first = false;
        } else {
            self.serializer.output.push(',');
        }
        value.serialize(self.serializer);
    }

    fn end(self) {
        self.serializer.output.push(']');
    }
}

pub struct JsonMapSerializer<'a> {
    serializer: &'a mut JsonSerializer,
    first: bool,
}

impl MapSerializer for JsonMapSerializer<'_> {
    type Serializer = JsonSerializer;

    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(&mut self, key: &K) {
        if self.first {
            self.first = false;
        } else {
            self.serializer.output.push(',');
        }
        key.serialize(self.serializer);
        self.serializer.output.push(':');
    }
    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(&mut self, value: &V) {
        value.serialize(self.serializer);
    }
    fn end(self) {
        self.serializer.output.push('}');
    }
}

#[test]
fn test_u32() {
    let mut json = JsonSerializer {
        output: String::new(),
    };
    std::hint::black_box((6_u32).serialize(&mut json));
    assert_eq!(json.output, "6");
}

#[test]
fn test_struct() {
    struct A {
        first: u32
    }
    let mut json = JsonSerializer {
        output: String::new(),
    };
    std::hint::black_box((A { first: 1 }).serialize(&mut json));
    assert_eq!(json.output, "{\"first\":1}");
}