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

use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::utils::try_to_url;

/// Concrete instances of MI generated during interpretation that may be used to specify post-interpretation behavior.
pub trait MetadataInformationResultElement: Debug {}

/// TODO
#[derive(Debug, Default, Clone)]
pub struct MetadataInformationResultElements {
    pub elements: Vec<Arc<dyn MetadataInformationResultElement>>,
}

/// TODO
#[derive(Debug, PartialEq)]
pub enum Stages {
    Pre,
    PreClientRequest,
    ClientRequest,
    PostClientRequest,
    PreOriginRequest,
    OriginRequest,
    PostOriginRequest,
    Source,
    PreOriginResponse,
    OriginResponse,
    PostOriginResponse,
    PreClientResponse,
    ClientResponse,
    PostClientResponse,
    Post,
}

/// TODO
#[derive(Debug)]
pub enum Error {
    InvalidType(String /* expected */, String /* actual */),
    InvalidMetadata(Box<dyn std::error::Error>),
    NoProcessor(String /* type for which processor is missing */),
    InvalidInput(Box<dyn std::error::Error>),
    AssertionFailure(Box<dyn std::error::Error>),
    NotImplemented(String),
    InvalidOutput,
    RuntimeError(Box<dyn std::error::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotImplemented(feature) => write!(f, "Feature {feature} is not yet implemented"),
            Error::InvalidType(_, _) => write!(f, "Invalid type"),
            Error::InvalidMetadata(_) => write!(f, "Invalid metadata"),
            Error::NoProcessor(_) => write!(f, "No processor"),
            Error::InvalidInput(e) => write!(f, "Invalid input: {e}"),
            Error::InvalidOutput => write!(f, "Invalid output"),
            Error::AssertionFailure(e) => write!(f, "Assertion failure: {e}"),
            Error::RuntimeError(e) => write!(f, "Runtime error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

pub struct FromableRequest<'a>(pub &'a http::Request<Vec<u8>>);

impl<'a> TryFrom<FromableRequest<'a>> for reqwest::Request {
    type Error = Error;

    fn try_from(value: FromableRequest) -> std::result::Result<Self, Self::Error> {
        let url = try_to_url(value.0.uri()).map_err(|e| Error::InvalidInput(e.into()))?;

        let mut request = reqwest::Request::new(reqwest::Method::GET, url);

        // Put in the headers.
        for (header_key, header_value) in value.0.headers() {
            request
                .headers_mut()
                .insert(header_key, header_value.clone());
        }

        // Configure the body.
        request.body_mut().replace(value.0.body().clone().into());

        Ok(request)
    }
}
