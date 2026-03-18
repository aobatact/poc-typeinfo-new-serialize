//! # poc-typeinfo-new-deser
//!
//! A proof-of-concept serialization framework that leverages Rust's nightly
//! [`type_info`] and [`try_as_dyn`] features to **automatically serialize any
//! `'static` type without proc macros**.
//!
//! ## How It Works
//!
//! Traditional serde relies on `#[derive(Serialize)]` proc macros to generate
//! serialization code for each type. This crate takes a fundamentally different
//! approach:
//!
//! 1. **Compile-time type introspection** via [`TypeSer`]: extracts field names,
//!    offsets, and vtables from [`TypeId`] at compile time using `std::mem::type_info`.
//! 2. **Blanket implementation** of [`Ser`]: all `T: 'static` types automatically
//!    implement `Ser<S>` for any serializer `S`.
//! 3. **Specialization** via [`SpecializedSer`] / [`SpecializedSerInner`]: types like
//!    `String`, `Vec<T>`, `Option<T>`, etc. use optimized implementations dispatched
//!    through `try_as_dyn`, which take priority over the generic blanket impl.
//!
//! ## Example
//!
//! ```ignore
//! use poc_typeinfo_new_deser::{Ser, json::JsonSerializer};
//!
//! struct Point { x: f64, y: f64 }
//!
//! let mut json = JsonSerializer::new_vec();
//! (Point { x: 1.0, y: 2.0 }).serialize(&mut json).unwrap();
//! assert_eq!(json.as_str(), r#"{"x":1,"y":2}"#);
//! ```

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

/// The core serialization trait, analogous to `serde::Serialize`.
///
/// A blanket implementation is provided for all `T: 'static` types. The blanket
/// impl uses the following dispatch order:
///
/// 1. If `T` implements [`SpecializedSer<S>`], use that (user-defined specialization).
/// 2. If `T` implements [`SpecializedSerInner<S>`], use that (crate-internal specialization).
/// 3. Otherwise, introspect `T` via [`TypeSer`] and serialize based on its structure
///    (primitive, struct, tuple, array, or reference).
pub trait Ser<S: Serializer> {
    /// Serializes `self` into the given serializer.
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

/// Allows downstream crates to provide a specialized serialization for a specific type.
///
/// When a type implements this trait, its [`specialized_serialize`](SpecializedSer::specialized_serialize)
/// method is used instead of the default reflection-based serialization. This is checked
/// at runtime via `try_as_dyn` and takes the highest priority in the dispatch chain.
pub trait SpecializedSer<S: Serializer> {
    /// Serializes `self` using the type-specific specialized logic.
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

/// Crate-internal counterpart of [`SpecializedSer`].
///
/// This trait provides specialized serialization for standard library types
/// (e.g. `String`, `Vec<T>`, `Option<T>`) without occupying the public
/// [`SpecializedSer`] slot, so downstream crates remain free to define their
/// own specializations via [`SpecializedSer`].
///
/// Checked after `SpecializedSer` but before the reflection-based fallback.
pub(crate) trait SpecializedSerInner<S: Serializer> {
    /// Serializes `self` using the crate-internal specialized logic.
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

/// A data format that can serialize any type implementing [`Ser<Self>`](Ser).
///
/// This trait defines the full set of serialization primitives (booleans, integers,
/// floats, strings, etc.) as well as compound type constructors (sequences, tuples,
/// maps, structs, and their variant forms). Implementations produce output in a
/// specific format (e.g. JSON, MessagePack, etc.).
///
/// See [`json::JsonSerializer`] for a reference implementation.
pub trait Serializer: Sized {
    /// The type returned on successful serialization.
    type Ok;
    /// The error type returned when serialization fails.
    type Error: SerializeError;
    /// State object for serializing a dynamically-sized sequence (`[T]`, `Vec<T>`, etc.).
    type SerializeSeq<'a>: SerializeSeq<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a fixed-size heterogeneous tuple.
    type SerializeTuple<'a>: SerializeTuple<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a named tuple struct (e.g. `struct Rgb(u8, u8, u8)`).
    type SerializeTupleStruct<'a>: SerializeTupleStruct<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a tuple variant of an enum.
    type SerializeTupleVariant<'a>: SerializeTupleVariant<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a key-value map.
    type SerializeMap<'a>: SerializeMap<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a named struct with named fields.
    type SerializeStruct<'a>: SerializeStruct<Serializer = Self>
    where
        Self: 'a;
    /// State object for serializing a struct variant of an enum.
    type SerializeStructVariant<'a>: SerializeStructVariant<Serializer = Self>
    where
        Self: 'a;

    /// Serializes a `bool` value.
    fn serialize_bool(&mut self, v: bool) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `i8` value.
    fn serialize_i8(&mut self, v: i8) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `i16` value.
    fn serialize_i16(&mut self, v: i16) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `i32` value.
    fn serialize_i32(&mut self, v: i32) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `i64` value.
    fn serialize_i64(&mut self, v: i64) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `i128` value.
    fn serialize_i128(&mut self, v: i128) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `u8` value.
    fn serialize_u8(&mut self, v: u8) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `u16` value.
    fn serialize_u16(&mut self, v: u16) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `u32` value.
    fn serialize_u32(&mut self, v: u32) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `u64` value.
    fn serialize_u64(&mut self, v: u64) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `u128` value.
    fn serialize_u128(&mut self, v: u128) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `f32` value.
    fn serialize_f32(&mut self, v: f32) -> Result<Self::Ok, Self::Error>;
    /// Serializes an `f64` value.
    fn serialize_f64(&mut self, v: f64) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `char` value.
    fn serialize_char(&mut self, v: char) -> Result<Self::Ok, Self::Error>;
    /// Serializes a string slice.
    fn serialize_str(&mut self, v: &str) -> Result<Self::Ok, Self::Error>;
    /// Serializes a byte slice.
    fn serialize_bytes(&mut self, v: &[u8]) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `None` value (e.g. `null` in JSON).
    fn serialize_none(&mut self) -> Result<Self::Ok, Self::Error>;
    /// Serializes a `Some(value)`.
    fn serialize_some<T: Ser<Self>>(&mut self, value: &T) -> Result<Self::Ok, Self::Error>;
    /// Serializes the unit type `()`.
    fn serialize_unit(&mut self) -> Result<Self::Ok, Self::Error>;
    /// Serializes a unit struct (e.g. `struct Unit;`).
    fn serialize_unit_struct(&mut self, name: &'static str) -> Result<Self::Ok, Self::Error>;
    /// Serializes a unit variant of an enum (e.g. `Color::Red`).
    fn serialize_unit_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error>;
    /// Serializes a newtype struct (e.g. `struct Inches(u32)`).
    fn serialize_newtype_struct<T: Ser<Self>>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    /// Serializes a newtype variant of an enum (e.g. `Value::Int(i32)`).
    fn serialize_newtype_variant<T: Ser<Self>>(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    /// Begins serializing a variable-length sequence. `len` is a hint if known.
    fn serialize_seq(&mut self, len: Option<usize>) -> Result<Self::SerializeSeq<'_>, Self::Error>;
    /// Begins serializing a fixed-length tuple of `len` elements.
    fn serialize_tuple(&mut self, len: usize) -> Result<Self::SerializeTuple<'_>, Self::Error>;
    /// Begins serializing a named tuple struct with `len` fields.
    fn serialize_tuple_struct(
        &mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct<'_>, Self::Error>;
    /// Begins serializing a tuple variant of an enum with `len` fields.
    fn serialize_tuple_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant<'_>, Self::Error>;
    /// Begins serializing a map. `len` is a hint if known.
    fn serialize_map(&mut self, len: Option<usize>) -> Result<Self::SerializeMap<'_>, Self::Error>;
    /// Begins serializing a named struct with `len` fields.
    fn serialize_struct(
        &mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct<'_>, Self::Error>;
    /// Begins serializing a struct variant of an enum with `len` fields.
    fn serialize_struct_variant(
        &mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant<'_>, Self::Error>;
}

/// Marker trait for serialization error types.
pub trait SerializeError {}

/// Returned by [`Serializer::serialize_seq`]. Serializes a variable-length sequence
/// element by element, then finalizes with [`end`](SerializeSeq::end).
pub trait SerializeSeq {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single element of the sequence.
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the sequence.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_tuple`]. Serializes a fixed-size tuple
/// element by element, then finalizes with [`end`](SerializeTuple::end).
pub trait SerializeTuple {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single element of the tuple.
    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the tuple.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_tuple_struct`]. Serializes a named tuple
/// struct field by field, then finalizes with [`end`](SerializeTupleStruct::end).
pub trait SerializeTupleStruct {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single field of the tuple struct.
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the tuple struct.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_tuple_variant`]. Serializes an enum's tuple
/// variant field by field, then finalizes with [`end`](SerializeTupleVariant::end).
pub trait SerializeTupleVariant {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single field of the tuple variant.
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the tuple variant.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_map`]. Serializes a key-value map entry by
/// entry, then finalizes with [`end`](SerializeMap::end).
pub trait SerializeMap {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a map key. Must be followed by a [`serialize_value`](SerializeMap::serialize_value) call.
    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Serializes a map value. Must be called after [`serialize_key`](SerializeMap::serialize_key).
    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &V,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Convenience method that serializes a key-value pair in one call.
    fn serialize_entry<K: Ser<Self::Serializer> + ?Sized, V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), <Self::Serializer as Serializer>::Error> {
        self.serialize_key(key)?;
        self.serialize_value(value)
    }
    /// Finishes serializing the map.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_struct`]. Serializes a named struct field by
/// field (with field names), then finalizes with [`end`](SerializeStruct::end).
pub trait SerializeStruct {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single named field of the struct.
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the struct.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Returned by [`Serializer::serialize_struct_variant`]. Serializes an enum's struct
/// variant field by field (with field names), then finalizes with
/// [`end`](SerializeStructVariant::end).
pub trait SerializeStructVariant {
    /// The parent serializer type.
    type Serializer: Serializer;
    /// Serializes a single named field of the struct variant.
    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), <Self::Serializer as Serializer>::Error>;
    /// Finishes serializing the struct variant.
    fn end(
        self,
    ) -> Result<<Self::Serializer as Serializer>::Ok, <Self::Serializer as Serializer>::Error>;
}

/// Vtable pointer metadata for `dyn Ser<S>`, used to construct fat pointers
/// from raw field pointers at runtime.
type DynSer<S> = DynMetadata<dyn Ser<S>>;

/// Maximum number of fields supported in a single struct or tuple.
/// Types with more fields will be truncated.
const MAX_FIELDS: usize = 20;

/// Compile-time type descriptor that determines how a type should be serialized.
///
/// Built from [`TypeId`] via `std::mem::type_info` at `const` evaluation time.
/// Each variant carries the metadata needed to serialize values of the described
/// type without any proc-macro-generated code.
enum TypeSer<S: 'static> {
    /// A primitive type (`bool`, `char`, integer, or float).
    Primitive(TypeKind),
    /// A tuple type (e.g. `(u32, f64)`), with field offsets and vtables.
    Tuple {
        fields: [MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS],
        len: usize,
    },
    /// A named struct with named fields, offsets, and vtables.
    Struct {
        fields: [MaybeUninit<SerFieldInfo<S>>; MAX_FIELDS],
        len: usize,
    },
    /// A fixed-length array `[T; N]`, with element size and vtable.
    Array {
        len: usize,
        elem: SerTypeInfo<S>,
    },
    // Slice {
    //     elem: SerTypeInfo<S>,
    // },
    /// A reference `&T`, with the pointee's size and vtable.
    Reference {
        referent: SerTypeInfo<S>,
    },
    /// A type whose structure is not directly supported by reflection
    /// (e.g. enums, unions). Falls back to `todo!()` at runtime.
    Other,
}

/// Metadata for a single field within a struct or tuple, obtained at compile time.
struct SerFieldInfo<S: 'static> {
    /// The field's name (empty string for tuple fields).
    name: &'static str,
    /// Byte offset of the field within the parent type's layout.
    offset: usize,
    /// Vtable pointer for `dyn Ser<S>`, used to serialize the field dynamically.
    vtable: DynSer<S>,
}

/// Metadata for a type referenced by an array element or a reference pointee.
struct SerTypeInfo<S: 'static> {
    /// The type's name (currently unused, reserved for diagnostics).
    #[allow(unused)]
    name: &'static str,
    /// The size of the type in bytes.
    size: usize,
    /// Vtable pointer for `dyn Ser<S>`, used to serialize values dynamically.
    vtable: DynSer<S>,
}

impl<S: 'static> SerFieldInfo<S> {
    /// Constructs a `&dyn Ser<S>` fat pointer to the field within `ptr`.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a value whose layout contains this field at `self.offset`.
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
    /// Constructs a `&dyn Ser<S>` fat pointer from a raw pointer to a value of this type.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a valid value of the type described by this `SerTypeInfo`.
    const unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
        let field_ptr = ptr as *const T as *const u8;
        unsafe {
            let fat_ptr =
                std::ptr::from_raw_parts::<dyn Ser<S>>(field_ptr as *const (), self.vtable);
            &*fat_ptr
        }
    }
}

/// Retrieves the `Ser<S>` vtable for the type identified by `type_id` at compile time.
///
/// Uses `TypeId::trait_info_of_trait_type_id` to look up the trait implementation
/// and extracts the vtable pointer via `transmute`.
///
/// # Panics
///
/// Panics at compile time if the type does not implement `Ser<S>`.
const fn get_reflect_vtable<S: Serializer + 'static>(type_id: TypeId) -> DynSer<S> {
    let trait_id = TypeId::of::<dyn Ser<S>>();
    match type_id.trait_info_of_trait_type_id(trait_id) {
        Some(t) => unsafe { std::mem::transmute(t.get_vtable()) },
        None => panic!("type does not implement Ser"),
    }
}

impl<S: Serializer + 'static> TypeSer<S> {
    /// Constructs a [`TypeSer`] descriptor from a [`TypeId`] at compile time.
    ///
    /// Inspects the type's [`TypeKind`] to determine its structure and collects
    /// field metadata (name, offset, vtable) for structs and tuples.
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

    /// Creates a [`TypeSer`] descriptor for type `T`, evaluated at compile time.
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

/// Dispatches serialization of a primitive value (`bool`, `char`, integer, or float)
/// to the appropriate `Serializer` method based on the [`TypeKind`].
///
/// # Safety
///
/// The raw pointer cast from `this` to the concrete primitive type is safe as long
/// as `type_kind` accurately describes `T`'s actual type. This is guaranteed because
/// `type_kind` is derived from `TypeId::of::<T>()` at compile time.
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
