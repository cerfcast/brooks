// brooks, Copyright 2026, Will Hawkins
//
// This file is part of brooks.

// This file is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::error;
use std::fmt::{Debug, Display};
use std::str::FromStr;

use http::{HeaderName, HeaderValue, Method, StatusCode};

use crate::cdni::gmdp::try_to_uri;

#[derive(Debug, Clone)]
pub enum ValueType {
    Url,
    HeaderName,
    HeaderValue,
    Status,
    Body,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Url => write!(f, "Url"),
            ValueType::HeaderName => write!(f, "Header name"),
            ValueType::HeaderValue => write!(f, "Header value"),
            ValueType::Body => write!(f, "Body"),
            ValueType::Status => write!(f, "Status"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    pub tpe: ValueType,
    pub value: Option<String>,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(value) = self.value.as_ref() {
            write!(f, "{}: {}", self.tpe, value)
        } else {
            write!(f, "{}", self.tpe)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    BadValue(Value),
    MissingValue(ValueType),
    BadStatus,
    InvalidMode,
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::BadValue(v) => write!(f, "Bad value: {v}"),
            Error::BadStatus => write!(f, "Bad Status"),
            Error::InvalidMode => write!(f, "Invalid mode"),
            Error::MissingValue(processable_request_response_value_type) => write!(
                f,
                "Missing value: {processable_request_response_value_type}"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrrType {
    Request,
    Response,
}

pub type Result<T> = std::result::Result<T, Error>;

/// A request that can be manipulated by interpretation processing stages.
pub trait Prr<Body>: Debug {
    // Header Manipulation and Access

    fn header_value(&self) -> Option<HeaderValue>;
    fn headers(&self) -> Vec<(String, HeaderValue)>;
    fn set_header_value(&mut self, header: &str, value: &str) -> Result<()>;
    fn clear_headers(&mut self) -> Result<()>;
    fn remove_header(&mut self, header: &str) -> Result<()>;
    fn add_header(&mut self, header: &str, value: &str) -> Result<()>;

    // URI Manipulation and Access

    fn url(&self) -> Result<url::Url>;
    fn set_url(&mut self, url: &url::Url) -> Result<()>;

    // Status Manipulation and Access

    fn set_status(&mut self, response: &u16) -> Result<()>;
    fn get_status(&self) -> Result<StatusCode>;

    // Body Manipulation and Access

    fn set_body(&mut self, body: &Body) -> Result<()>;
    fn get_body(&self) -> Result<&Body>;

    // Method Manipulation and Access

    fn set_method(&mut self, method: &Method) -> Result<()>;
    fn get_method(&self) -> Method;

    /// Get the type of processable request/response.
    fn tpe(&self) -> PrrType;
}

impl TryFrom<&dyn Prr<Vec<u8>>> for reqwest::Request {
    type Error = Error;

    fn try_from(value: &dyn Prr<Vec<u8>>) -> std::result::Result<Self, Self::Error> {
        if value.tpe() != PrrType::Request {
            return Err(Error::InvalidMode);
        }

        let url = value
            .url()
            .map_err(|_| Error::MissingValue(ValueType::Url))?;

        let mut request = reqwest::Request::new(reqwest::Method::GET, url);

        // Put in the headers.
        for (header_key, header_value) in value.headers() {
            request.headers_mut().insert(
                HeaderName::from_str(&header_key).map_err(|_| {
                    Error::BadValue(Value {
                        tpe: ValueType::HeaderName,
                        value: Some(header_key.clone()),
                    })
                })?,
                header_value,
            );
        }

        // Configure the body.
        request.body_mut().replace(
            value
                .get_body()
                .map_err(|_| Error::MissingValue(ValueType::Body))?
                .clone()
                .into(),
        );

        Ok(request)
    }
}

impl TryFrom<&dyn Prr<Vec<u8>>> for http::Request<Vec<u8>> {
    type Error = Error;

    fn try_from(value: &dyn Prr<Vec<u8>>) -> std::result::Result<Self, Self::Error> {
        if value.tpe() != PrrType::Request {
            return Err(Error::InvalidMode);
        }

        let mut request_builder = http::request::Builder::new();

        // Put in the headers.
        for (header_key, header_value) in value.headers() {
            request_builder = request_builder.header(header_key, header_value);
        }

        let url = value
            .url()
            .map_err(|_| Error::MissingValue(ValueType::Url))?;

        // Set the URI.
        request_builder = request_builder.uri(try_to_uri(&url).map_err(|_| {
            Error::BadValue(Value {
                tpe: ValueType::Url,
                value: Some(url.to_string()),
            })
        })?);

        // Configure the body.
        request_builder
            .body(
                value
                    .get_body()
                    .map_err(|_| Error::MissingValue(ValueType::Body))?
                    .clone(),
            )
            .map_err(|_| {
                Error::BadValue(Value {
                    tpe: ValueType::Body,
                    value: None,
                })
            })
    }
}
impl TryFrom<&dyn Prr<Vec<u8>>> for http::Response<Vec<u8>> {
    type Error = Error;

    fn try_from(value: &dyn Prr<Vec<u8>>) -> std::result::Result<Self, Self::Error> {
        if value.tpe() != PrrType::Response {
            return Err(Error::InvalidMode);
        }

        let mut response_builder = http::response::Builder::new();

        // Put in the headers.
        for (header_key, header_value) in value.headers() {
            response_builder = response_builder.header(header_key, header_value);
        }

        // Set the status.
        response_builder =
            response_builder.status(value.get_status().map_err(|_| Error::BadStatus)?);

        // Configure the body.
        response_builder
            .body(
                value
                    .get_body()
                    .map_err(|_| Error::MissingValue(ValueType::Body))?
                    .clone(),
            )
            .map_err(|_| {
                Error::BadValue(Value {
                    tpe: ValueType::Body,
                    value: None,
                })
            })
    }
}
