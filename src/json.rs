//! A minimal JSON [`Serializer`] implementation.
//!
//! Writes JSON output to any [`std::io::Write`] destination. Provides
//! convenience methods when backed by a `Vec<u8>` for in-memory serialization.
//!
//! # Example
//!
//! ```ignore
//! use poc_typeinfo_new_deser::{Ser, json::JsonSerializer};
//!
//! let mut ser = JsonSerializer::new_vec();
//! (42_u32).serialize(&mut ser).unwrap();
//! assert_eq!(ser.as_str(), "42");
//! ```

use super::*;
use std::io::Write;

/// A JSON serializer that writes output to a [`Write`] destination.
///
/// Use [`JsonSerializer::new`] to wrap any writer, or [`JsonSerializer::new_vec`]
/// for convenient in-memory serialization to a `Vec<u8>`.
pub struct JsonSerializer<W: Write> {
    writer: W,
}

impl<W: Write> JsonSerializer<W> {
    /// Creates a new `JsonSerializer` that writes JSON to the given writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Consumes the serializer and returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl JsonSerializer<Vec<u8>> {
    /// Creates a new `JsonSerializer` backed by an empty `Vec<u8>`.
    pub fn new_vec() -> Self {
        Self { writer: Vec::new() }
    }

    /// Consumes the serializer and returns the underlying byte buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.writer
    }

    /// Returns the serialized output as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.writer
    }

    /// Returns the serialized output as a UTF-8 string slice.
    ///
    /// # Panics
    ///
    /// Panics if the output is not valid UTF-8 (should not happen with well-formed JSON).
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.writer).unwrap()
    }

    /// Consumes the serializer and returns the output as a `String`.
    ///
    /// # Panics
    ///
    /// Panics if the output is not valid UTF-8.
    pub fn into_string(self) -> String {
        String::from_utf8(self.writer).unwrap()
    }
}

/// Errors that can occur during JSON serialization.
#[derive(Debug)]
pub enum JsonSerializeError {
    /// An I/O error from the underlying writer.
    Io(std::io::Error),
}

impl SerializeError for JsonSerializeError {}

impl From<std::io::Error> for JsonSerializeError {
    fn from(err: std::io::Error) -> Self {
        JsonSerializeError::Io(err)
    }
}

impl<W: Write> Serializer for JsonSerializer<W> {
    type Ok = ();
    type Error = JsonSerializeError;
    type SerializeSeq<'a>
        = JsonSerializeSeq<'a, W>
    where
        W: 'a;
    type SerializeTuple<'a>
        = JsonSerializeTuple<'a, W>
    where
        W: 'a;
    type SerializeTupleStruct<'a>
        = JsonSerializeTupleStruct<'a, W>
    where
        W: 'a;
    type SerializeTupleVariant<'a>
        = JsonSerializeTupleVariant<'a, W>
    where
        W: 'a;
    type SerializeMap<'a>
        = JsonSerializeMap<'a, W>
    where
        W: 'a;
    type SerializeStruct<'a>
        = JsonSerializeStruct<'a, W>
    where
        W: 'a;
    type SerializeStructVariant<'a>
        = JsonSerializeStructVariant<'a, W>
    where
        W: 'a;

    fn serialize_bool(&mut self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_all(if v { b"true" } else { b"false" })
            .map_err(Into::into)
    }

    fn serialize_i8(&mut self, v: i8) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_i16(&mut self, v: i16) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_i32(&mut self, v: i32) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_i64(&mut self, v: i64) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_i128(&mut self, v: i128) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_u8(&mut self, v: u8) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_u16(&mut self, v: u16) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_u32(&mut self, v: u32) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_u64(&mut self, v: u64) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_u128(&mut self, v: u128) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_f32(&mut self, v: f32) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_f64(&mut self, v: f64) -> Result<Self::Ok, Self::Error> {
        write!(&mut self.writer, "{}", v).map_err(Into::into)
    }

    fn serialize_char(&mut self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(&mut self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"\"")?;
        for c in v.chars() {
            match c {
                '\\' => self.writer.write_all(b"\\\\")?,
                '"' => self.writer.write_all(b"\\\"")?,
                '\n' => self.writer.write_all(b"\\n")?,
                '\r' => self.writer.write_all(b"\\r")?,
                '\t' => self.writer.write_all(b"\\t")?,
                _ => {
                    let mut buf = [0u8; 4];
                    let encoded = c.encode_utf8(&mut buf);
                    self.writer.write_all(encoded.as_bytes())?;
                }
            }
        }
        self.writer.write_all(b"\"")?;
        Ok(())
    }

    fn serialize_bytes(&mut self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"[")?;
        let mut first = true;
        for &byte in v {
            if first {
                first = false;
            } else {
                self.writer.write_all(b",")?;
            }
            write!(&mut self.writer, "{}", byte)?;
        }
        self.writer.write_all(b"]")?;
        Ok(())
    }

    fn serialize_none(&mut self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"null").map_err(Into::into)
    }

    fn serialize_some<T: Ser<Self>>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(&mut self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"null").map_err(Into::into)
    }

    fn serialize_unit_struct(&mut self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        &mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: Ser<Self>>(
        &mut self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Ser<Self>>(
        &mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"{")?;
        self.serialize_str(variant)?;
        self.writer.write_all(b":")?;
        value.serialize(self)?;
        self.writer.write_all(b"}")?;
        Ok(())
    }

    fn serialize_seq(
        &mut self,
        _len: Option<usize>,
    ) -> Result<Self::SerializeSeq<'_>, Self::Error> {
        self.writer.write_all(b"[")?;
        Ok(JsonSerializeSeq {
            serializer: self,
            first: true,
        })
    }

    fn serialize_tuple(&mut self, _len: usize) -> Result<Self::SerializeTuple<'_>, Self::Error> {
        self.writer.write_all(b"[")?;
        Ok(JsonSerializeTuple {
            serializer: self,
            first: true,
        })
    }

    fn serialize_tuple_struct(
        &mut self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct<'_>, Self::Error> {
        self.writer.write_all(b"[")?;
        Ok(JsonSerializeTupleStruct {
            serializer: self,
            first: true,
        })
    }

    fn serialize_tuple_variant(
        &mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant<'_>, Self::Error> {
        self.writer.write_all(b"{")?;
        self.serialize_str(variant)?;
        self.writer.write_all(b":[")?;
        Ok(JsonSerializeTupleVariant {
            serializer: self,
            first: true,
        })
    }

    fn serialize_map(
        &mut self,
        _len: Option<usize>,
    ) -> Result<Self::SerializeMap<'_>, Self::Error> {
        self.writer.write_all(b"{")?;
        Ok(JsonSerializeMap {
            serializer: self,
            first: true,
        })
    }

    fn serialize_struct(
        &mut self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct<'_>, Self::Error> {
        self.writer.write_all(b"{")?;
        Ok(JsonSerializeStruct {
            serializer: self,
            first: true,
        })
    }

    fn serialize_struct_variant(
        &mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant<'_>, Self::Error> {
        self.writer.write_all(b"{")?;
        self.serialize_str(variant)?;
        self.writer.write_all(b":{")?;
        Ok(JsonSerializeStructVariant {
            serializer: self,
            first: true,
        })
    }
}

// --- SerializeSeq ---

/// JSON state for serializing a sequence (e.g. `[1,2,3]`).
pub struct JsonSerializeSeq<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeSeq for JsonSerializeSeq<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"]")?;
        Ok(())
    }
}

// --- SerializeTuple ---

/// JSON state for serializing a tuple (e.g. `[1,"a",true]`).
pub struct JsonSerializeTuple<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeTuple for JsonSerializeTuple<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_element<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"]")?;
        Ok(())
    }
}

// --- SerializeTupleStruct ---

/// JSON state for serializing a tuple struct (e.g. `[1,2,3]`).
pub struct JsonSerializeTupleStruct<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeTupleStruct for JsonSerializeTupleStruct<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"]")?;
        Ok(())
    }
}

// --- SerializeTupleVariant ---

/// JSON state for serializing a tuple variant (e.g. `{"Variant":[1,2]}`).
pub struct JsonSerializeTupleVariant<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeTupleVariant for JsonSerializeTupleVariant<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"]}")?;
        Ok(())
    }
}

// --- SerializeMap ---

/// JSON state for serializing a map (e.g. `{"key":"value"}`).
pub struct JsonSerializeMap<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeMap for JsonSerializeMap<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_key<K: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &K,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        key.serialize(self.serializer)?;
        self.serializer.writer.write_all(b":")?;
        Ok(())
    }

    fn serialize_value<V: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        value: &V,
    ) -> Result<(), JsonSerializeError> {
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"}")?;
        Ok(())
    }
}

// --- SerializeStruct ---

/// JSON state for serializing a struct (e.g. `{"field":value}`).
pub struct JsonSerializeStruct<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeStruct for JsonSerializeStruct<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        self.serializer.serialize_str(key)?;
        self.serializer.writer.write_all(b":")?;
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"}")?;
        Ok(())
    }
}

// --- SerializeStructVariant ---

/// JSON state for serializing a struct variant (e.g. `{"Variant":{"field":value}}`).
pub struct JsonSerializeStructVariant<'a, W: Write> {
    serializer: &'a mut JsonSerializer<W>,
    first: bool,
}

impl<W: Write> SerializeStructVariant for JsonSerializeStructVariant<'_, W> {
    type Serializer = JsonSerializer<W>;

    fn serialize_field<T: Ser<Self::Serializer> + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), JsonSerializeError> {
        if self.first {
            self.first = false;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        self.serializer.serialize_str(key)?;
        self.serializer.writer.write_all(b":")?;
        value.serialize(self.serializer)
    }

    fn end(self) -> Result<(), JsonSerializeError> {
        self.serializer.writer.write_all(b"}}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32() {
        let mut json = JsonSerializer::new_vec();
        (6_u32).serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "6");
    }

    #[test]
    fn test_f32() {
        let mut json = JsonSerializer::new_vec();
        (6.5_f32).serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "6.5");
    }

    #[test]
    fn test_char() {
        let mut json = JsonSerializer::new_vec();
        ('a').serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "\"a\"");
    }

    #[test]
    fn test_struct() {
        struct A {
            #[allow(unused)]
            first: u32,
        }
        let mut json = JsonSerializer::new_vec();
        (A { first: 1 }).serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "{\"first\":1}");
    }

    #[test]
    fn test_str() {
        let mut json = JsonSerializer::new_vec();
        ("aaa").serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "\"aaa\"");
    }

    #[test]
    fn test_str_escape() {
        let mut json = JsonSerializer::new_vec();
        ("aa\"a").serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "\"aa\\\"a\"");
    }

    #[test]
    fn test_array() {
        let mut json = JsonSerializer::new_vec();
        ([0, 1, 2]).serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "[0,1,2]");
    }

    #[test]
    fn test_slice() {
        let mut json = JsonSerializer::new_vec();
        let v = [1, 2, 3];
        let s: &[i32] = &v;
        s.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "[1,2,3]");
    }

    #[test]
    fn test_vec() {
        let mut json = JsonSerializer::new_vec();
        let v = vec![10, 20, 30];
        v.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "[10,20,30]");
    }

    #[test]
    fn test_vec_empty() {
        let mut json = JsonSerializer::new_vec();
        let v: Vec<u32> = vec![];
        v.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "[]");
    }

    #[test]
    fn test_ref() {
        let mut json = JsonSerializer::new_vec();
        let ref_int = &&42;
        ref_int.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "42");
    }

    #[test]
    fn test_box() {
        let mut json = JsonSerializer::new_vec();
        let boxed_int = Box::new(42);
        boxed_int.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "42");
    }

    #[test]
    fn test_option() {
        let mut json = JsonSerializer::new_vec();
        let opt_int = Some(42);
        opt_int.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "42");

        let opt_none: Option<u32> = None;
        let mut json_none = JsonSerializer::new_vec();
        opt_none.serialize(&mut json_none).unwrap();
        assert_eq!(json_none.as_str(), "null");
    }

    #[test]
    fn test_nonzero_u32() {
        let mut json = JsonSerializer::new_vec();
        let val = std::num::NonZeroU32::new(42).unwrap();
        val.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "42");
    }

    #[test]
    fn test_nonzero_i64() {
        let mut json = JsonSerializer::new_vec();
        let val = std::num::NonZeroI64::new(-100).unwrap();
        val.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), "-100");
    }

    #[test]
    fn test_nonzero_in_struct() {
        struct Config {
            #[allow(unused)]
            port: std::num::NonZeroU16,
            #[allow(unused)]
            retries: std::num::NonZeroU8,
        }
        let mut json = JsonSerializer::new_vec();
        let cfg = Config {
            port: std::num::NonZeroU16::new(8080).unwrap(),
            retries: std::num::NonZeroU8::new(3).unwrap(),
        };
        cfg.serialize(&mut json).unwrap();
        assert_eq!(json.as_str(), r#"{"port":8080,"retries":3}"#);
    }

    #[test]
    fn test_hashmap() {
        use std::collections::HashMap;
        let mut json = JsonSerializer::new_vec();
        let mut map = HashMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        map.serialize(&mut json).unwrap();
        // Note: HashMap does not guarantee order, so we check both possibilities.
        let s = json.as_str();
        assert!(
            s == "{\"a\":1,\"b\":2}" || s == "{\"b\":2,\"a\":1}",
            "Unexpected JSON output: {}",
            s
        );
    }
}
