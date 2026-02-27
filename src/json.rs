use super::*;
use std::fmt::Write;

pub struct JsonSerializer {
    output: String,
}
impl JsonSerializer {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn into_string(self) -> String {
        self.output
    }

    pub fn as_str(&self) -> &str {
        &self.output
    }
}

impl Serializer for JsonSerializer {
    type Sequence<'a> = JsonSequenceSerializer<'a>;
    type Map<'a> = JsonMapSerializer<'a>;
    type Struct<'a> = JsonStructSerializer<'a>;

    fn serialize_str(&mut self, value: &str) {
        self.output.reserve(value.len());
        self.output.push('"');
        for c in value.chars() {
            match c {
                '\\' => self.output.push_str("\\\\"),
                '"' => self.output.push_str("\\\""),
                '\n' => self.output.push_str("\\n"),
                '\r' => self.output.push_str("\\r"),
                '\t' => self.output.push_str("\\t"),
                _ => self.output.push(c),
            }
        }
        self.output.push('"');
    }

    fn serialize_i8(&mut self, value: i8) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_u8(&mut self, value: u8) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_i16(&mut self, value: i16) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_u16(&mut self, value: u16) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_i32(&mut self, value: i32) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_u32(&mut self, value: u32) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_i64(&mut self, value: i64) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_u64(&mut self, value: u64) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_i128(&mut self, value: i128) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_u128(&mut self, value: u128) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_bool(&mut self, value: bool) {
        write!(&mut self.output, "{}", if value { "true" } else { "false" }).unwrap();
    }

    fn serialize_f32(&mut self, value: f32) {
        write!(&mut self.output, "{}", value).unwrap();
    }

    fn serialize_f64(&mut self, value: f64) {
        write!(&mut self.output, "{}", value).unwrap();
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

    fn serialize_struct(&mut self) -> Self::Struct<'_> {
        self.output.push('{');
        JsonStructSerializer {
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

pub struct JsonStructSerializer<'a> {
    serializer: &'a mut JsonSerializer,
    first: bool,
}

impl StructSerializer for JsonStructSerializer<'_> {
    type Serializer = JsonSerializer;

    fn serialize_struct_name(&mut self, _struct_name: &str) {
        // JSON doesn't have a concept of struct name, so we can ignore it.
    }

    fn serialize_field_name(&mut self, field_name: &str) {
        if self.first {
            self.first = false;
        } else {
            self.serializer.output.push(',');
        }
        self.serializer.serialize_str(field_name);
        self.serializer.output.push(':');
    }

    fn serialize_field_value<T: Ser<Self::Serializer>>(&mut self, value: &T) {
        value.serialize(self.serializer);
    }

    fn end(self) {
        self.serializer.output.push('}');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32() {
        let mut json = JsonSerializer::new();
        (6_u32).serialize(&mut json);
        assert_eq!(json.as_str(), "6");
    }

    #[test]
    fn test_struct() {
        struct A {
            first: u32,
        }
        let mut json = JsonSerializer::new();
        (A { first: 1 }).serialize(&mut json);
        assert_eq!(json.as_str(), "{\"first\":1}");
    }

    #[test]
    fn test_str() {
        let mut json = JsonSerializer::new();
        ("aaa").serialize(&mut json);
        assert_eq!(json.as_str(), "\"aaa\"");
    }

    #[test]
    fn test_array() {
        let mut json = JsonSerializer::new();
        ([0, 1, 2]).serialize(&mut json);
        assert_eq!(json.as_str(), "[0,1,2]");
    }
}
