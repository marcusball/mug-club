extern crate serde;

use serde::ser::{Serialize, SerializeStruct, Serializer};

mod util;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
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
    pub data: Option<ApiResponseEnvelope<T>>,
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
    pub fn from(data: Option<T>) -> ApiResponse<T> {
        ApiResponse {
            status: ResponseStatus::Success,
            data: data.map(|data| ApiResponseEnvelope(data)),
            messages: None,
        }
    }

    pub fn success(data: T) -> ApiResponse<T> {
        ApiResponse {
            status: ResponseStatus::Success,
            data: Some(ApiResponseEnvelope(data)),
            messages: None,
        }
    }

    #[allow(dead_code)]
    pub fn fail(data: T) -> ApiResponse<T> {
        ApiResponse {
            status: ResponseStatus::Fail,
            data: Some(ApiResponseEnvelope(data)),
            messages: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(data: T) -> ApiResponse<T> {
        ApiResponse {
            status: ResponseStatus::Error,
            data: Some(ApiResponseEnvelope(data)),
            messages: None,
        }
    }

    pub fn with_status(mut self, status: ResponseStatus) -> ApiResponse<T> {
        self.status = status;
        self
    }

    #[allow(dead_code)]
    pub fn data(mut self, data: T) -> ApiResponse<T> {
        self.data = Some(ApiResponseEnvelope(data));
        self
    }

    pub fn add_message(mut self, message: String) -> ApiResponse<T> {
        if self.messages.is_none() {
            self.messages = Some(Vec::new());
        }

        self.messages.as_mut().unwrap().push(message);
        self
    }
}
