extern crate serde;
extern crate serde_derive;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{self, Impossible, Serialize, SerializeStruct, SerializeTupleStruct, Serializer};
use std::fmt::{self, Display};

/// Get the name type name of the object, or the value of the `#[serde(rename="my_name")]` attribute.
///
/// This is used to get the name of the field that will contain a struct in the API response.
///
/// See: https://stackoverflow.com/a/46613908/451726
pub fn response_object_name<T: Serialize>(t: &T) -> &'static str {
    #[derive(Debug)]
    struct NotStruct;
    type Result<T> = std::result::Result<T, NotStruct>;
    impl std::error::Error for NotStruct {
        fn description(&self) -> &str {
            "not struct"
        }
    }
    impl Display for NotStruct {
        fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
            unimplemented!()
        }
    }
    impl ser::Error for NotStruct {
        fn custom<T: Display>(_msg: T) -> Self {
            NotStruct
        }
    }

    struct TypeName;
    impl Serializer for TypeName {
        type Ok = &'static str;
        type Error = NotStruct;
        type SerializeSeq = Impossible<Self::Ok, Self::Error>;
        type SerializeTuple = Impossible<Self::Ok, Self::Error>;
        type SerializeTupleStruct = Struct;
        type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
        type SerializeMap = Impossible<Self::Ok, Self::Error>;
        type SerializeStruct = Struct;
        type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
        fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_u8(self, _v: u8) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_char(self, _v: char) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_str(self, _v: &str) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_none(self) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_unit(self) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
            Ok(name)
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok> {
            Ok(name)
        }
        fn serialize_newtype_variant<T: ?Sized + Serialize>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok> {
            Err(NotStruct)
        }
        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
            Err(NotStruct)
        }
        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
            Err(NotStruct)
        }
        fn serialize_tuple_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct> {
            Ok(Struct(name))
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant> {
            Err(NotStruct)
        }
        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
            Err(NotStruct)
        }
        fn serialize_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct> {
            Ok(Struct(name))
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant> {
            Err(NotStruct)
        }
    }

    struct Struct(&'static str);
    impl SerializeStruct for Struct {
        type Ok = &'static str;
        type Error = NotStruct;
        fn serialize_field<T: ?Sized + Serialize>(
            &mut self,
            _key: &'static str,
            _value: &T,
        ) -> Result<()> {
            Ok(())
        }
        fn end(self) -> Result<Self::Ok> {
            Ok(self.0)
        }
    }
    impl SerializeTupleStruct for Struct {
        type Ok = &'static str;
        type Error = NotStruct;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<()> {
            Ok(())
        }
        fn end(self) -> Result<Self::Ok> {
            Ok(self.0)
        }
    }

    t.serialize(TypeName).unwrap()
}
