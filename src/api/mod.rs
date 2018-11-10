extern crate serde;

use serde::ser::{self, Impossible, Serialize, SerializeStruct, SerializeTupleStruct, Serializer};

mod util;

#[derive(Serialize)]
pub enum ResponseStatus {
    Success,
    Error,
    Fail,
}

/// Envelope type for API responses
///
/// When serializing as JSON, this will result in a JSON object with the inner object returned
/// using a field name that is specified by the `#[serde(rename="field_name")]` attribute.
/// If no such attribute is specified on `T`, the field will use the type name of `T`.
pub struct ApiResponseEnvelope<T: Serialize>(T);

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub status: ResponseStatus,
    pub data: ApiResponseEnvelope<T>,
    pub messages: Option<Vec<String>>,
}

impl<T> Serialize for ApiResponseEnvelope<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("ApiResponseEnvelope", 1)?;
        state.serialize_field(util::response_object_name(&self.0), &self.0)?;
        state.end()
    }
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn new(data: T) -> ApiResponse<T> {
        ApiResponse {
            status: ResponseStatus::Success,
            data: ApiResponseEnvelope(data),
            messages: None,
        }
    }
}
