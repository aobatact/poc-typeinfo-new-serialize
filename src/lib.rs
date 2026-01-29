#![feature(try_as_dyn)]
#![feature(type_info)]
use std::mem::type_info::Type;

pub trait Ser {
    fn serialize<S: Serializer + 'static>(&self, serializer: &mut S);
}

pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S);
}

impl<T: Ser> SpecializedSer<JsonSerializer> for Option<T>{
    fn specialized_serialize(&self, serializer: &mut JsonSerializer) {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

pub trait Serializer {
    fn serialize_str(&mut self, value: &str);
    fn serialize_i8(&mut self, value: i8);
    fn serialize_u8(&mut self, value: u8);
    fn serialize_i16(&mut self, value: i16);
    fn serialize_u16(&mut self, value: u16);
    fn serialize_i32(&mut self, value: i32);
    fn serialize_u32(&mut self, value: u32);
    fn serialize_i64(&mut self, value: i64);
    fn serialize_u64(&mut self, value: u64);
    fn serialize_bool(&mut self, value: bool);
    fn serialize_f32(&mut self, value: f32);
    fn serialize_f64(&mut self, value: f64);
    fn serialize_unit(&mut self);
    fn serialize_some<T: Ser>(&mut self, value: &T);
    fn serialize_none(&mut self);
    fn serialize_seq<T: Ser, I: Iterator<Item = T>>(&mut self, values: I);
    fn serialize_map<K: Ser, V: Ser, I: Iterator<Item = (K, V)>>(&mut self, entries: I);
}

impl<T: 'static> Ser for T {
    fn serialize<S: Serializer + 'static>(&self, serializer: &mut S) {
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer);
        } else {
            let type_val  = const { Type::of::<T>() };
            match type_val.kind {
                std::mem::type_info::TypeKind::Tuple(tuple) => {
                    tuple.fields.iter().for_each(|field|{
                        unsafe {
                            let field_ptr = (self as *const T as *const u8).add(field.offset) as *const ();
                            todo!("問題点: 現状、type id から Ser を取得する方法がない…")
                        }
                    });
                },
                std::mem::type_info::TypeKind::Array(array) => {
                    unsafe {
                        let len = array.len;
                        todo!("現状、type id から Ser を取得する方法がない…")
                    }
                },
                std::mem::type_info::TypeKind::Bool(_bool) => {
                    unsafe {
                        let b = *(self as *const T as *const bool);
                        serializer.serialize_bool(b);
                    }
                },
                std::mem::type_info::TypeKind::Char(_char) => {
                    unsafe {
                        let c = *(self as *const T as *const char);
                        let mut buf = [0; 4];
                        let s = c.encode_utf8(&mut buf);
                        serializer.serialize_str(s);
                    }
                },
                std::mem::type_info::TypeKind::Int(int) => {
                    if int.signed {
                        unsafe { match int.bits {
                            8 => serializer.serialize_i8(*(self as *const T as *const i8)),
                            16 => serializer.serialize_i16(*(self as *const T as *const i16)),
                            32 => serializer.serialize_i32(*(self as *const T as *const i32)),
                            64 => serializer.serialize_i64(*(self as *const T as *const i64)),
                            _ => unreachable!(),
                        }}
                    } else {
                        unsafe { match int.bits {
                            8 => serializer.serialize_u8(*(self as *const T as *const u8)),
                            16 => serializer.serialize_u16(*(self as *const T as *const u16)),
                            32 => serializer.serialize_u32(*(self as *const T as *const u32)),
                            64 => serializer.serialize_u64(*(self as *const T as *const u64)),
                            _ => unreachable!(),
                        }}
                    }
                },
                std::mem::type_info::TypeKind::Float(float) => {
                    unsafe {
                        match float.bits {
                            32 => serializer.serialize_f32(*(self as *const T as *const f32)),
                            64 => serializer.serialize_f64(*(self as *const T as *const f64)),
                            _ => unreachable!(),
                        }
                    }
                },
                std::mem::type_info::TypeKind::Str(str) => todo!(),
                std::mem::type_info::TypeKind::Reference(reference) => todo!(),
                std::mem::type_info::TypeKind::Other => todo!(),
                _ => todo!(),
            }
        }
        
    }
} 

pub struct JsonSerializer {
    output: String,
}

impl Serializer for JsonSerializer {
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

    fn serialize_some<T: Ser>(&mut self, value: &T) {
        value.serialize(self);
    }

    fn serialize_none(&mut self) {
        self.output.push_str("null");
    }

    fn serialize_seq<T: Ser, I: Iterator<Item = T>>(&mut self, values: I) {
        self.output.push('[');
        let mut first = true;
        for value in values {
            if !first {
                self.output.push(',');
            }
            first = false;
            value.serialize(self);
        }
        self.output.push(']');
    }

    fn serialize_map<K: Ser, V: Ser, I: Iterator<Item = (K, V)>>(&mut self, entries: I) {
        self.output.push('{');
        let mut first = true;
        for (key, value) in entries {
            if !first {
                self.output.push(',');  
            }
            first = false;
            key.serialize(self);
            self.output.push(':');
            value.serialize(self);
        }
        self.output.push('}');
    }
}

fn main() {
    let mut json = JsonSerializer {
        output: String::new()
    };
    std::hint::black_box((6_u32).serialize(&mut json));
    println!("{}", json.output);
}