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

//! Generic Metadata Processing

use std::fmt::{Debug, Display};
use std::str::FromStr;

use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, uri};

use crate::cdni::gmd::spec::TypedGenericMetadata;
use crate::cdni::md::verify::CdniVerificationKey;
use crate::tools::prr;

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

#[derive(Debug)]
pub enum Error {
    InvalidType(String /* expected */, String /* actual */),
    InvalidMetadata(Box<dyn std::error::Error>),
    NoProcessor(String /* type for which processor is missing */),
    InvalidInput(Box<dyn std::error::Error>),
    AssertionFailure(Box<dyn std::error::Error>),
    InvalidOutput,
    RuntimeError(Box<dyn std::error::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

/// The context for the interpretation of a validated representation of a given piece of Metadata Information.
///
///
/// The `cookie` (of user-specified type `C`) can be used to allow access to other data
/// an interpretation of the MI may update.
///
/// TODO: Determine whether the runtime can be factored out (made generic).
#[derive(Debug)]
pub struct InterpreterContext<C> {
    pub request: Option<Request<Vec<u8>>>,
    pub response: Option<reqwest::Response>,
    pub runtime: tokio::runtime::Runtime,
    pub cookie: C,
}

pub type AnalysisResult<Ck, Error> = Result<(Ck, CdniVerificationKey), Box<Error>>;

pub type InterpretationResult<Ck, Res, Error> = Result<(Ck, Res), Box<Error>>;

/// Interpret a validated representation of a given piece of Metadata Information.
pub trait Interpreter<Ck, Res, Error>: Debug {
    /// Perform the semantics of the MI.
    fn interpret(&self, input: Ck) -> InterpretationResult<Ck, Res, Error>;

    /// Determine whether this MI should be interpreted at this stage in processing of stages.
    fn stage(&self, stage: Stages) -> bool;
}

/// Generate a validated representation of a given piece of Metadata Information, if possible.
pub trait Verifier<Ck, Error>: Debug {
    fn analyze(&self, v: &TypedGenericMetadata<()>, input: Ck) -> AnalysisResult<Ck, Error>;

    fn type_name(&self, candidate: &str) -> bool;
}

pub fn try_to_url<A: ToString>(from: A) -> Result<url::Url, url::ParseError> {
    url::Url::from_str(&from.to_string())
}

pub fn try_to_uri<A: ToString>(from: A) -> Result<http::Uri, uri::InvalidUri> {
    http::Uri::from_str(&from.to_string())
}

#[derive(Debug)]
pub struct ProcessedRequestResponse {
    pub oheaders: HeaderMap<HeaderValue>,
    pub obody: Vec<u8>,
    pub ourl: url::Url,
    pub ostatus: Option<StatusCode>,
    pub omethod: Method,
    pub tpe: prr::PrrType,
}

impl ProcessedRequestResponse {
    pub fn new_request(url: &url::Url, method: &Method) -> Self {
        ProcessedRequestResponse {
            oheaders: HeaderMap::<HeaderValue>::default(),
            obody: vec![],
            ourl: url.clone(),
            ostatus: None,
            tpe: prr::PrrType::Request,
            omethod: method.clone(),
        }
    }

    pub fn new_response(url: &url::Url, method: &Method) -> Self {
        ProcessedRequestResponse {
            oheaders: HeaderMap::<HeaderValue>::default(),
            obody: vec![],
            ourl: url.clone(),
            ostatus: None,
            tpe: prr::PrrType::Response,
            omethod: method.clone(),
        }
    }
}

impl TryFrom<&http::Request<Vec<u8>>> for ProcessedRequestResponse {
    type Error = url::ParseError;

    fn try_from(value: &http::Request<Vec<u8>>) -> Result<Self, Self::Error> {
        let mut result =
            ProcessedRequestResponse::new_request(&try_to_url(value.uri())?, value.method());

        let mut headers = HeaderMap::<HeaderValue>::default();
        for (k, v) in value.headers() {
            headers.insert(k, v.clone());
        }

        result.oheaders = headers;
        result.ostatus = None;
        result.obody = value.body().clone();

        Ok(result)
    }
}

impl prr::Prr<Vec<u8>> for ProcessedRequestResponse {
    fn header_value(&self) -> Option<HeaderValue> {
        todo!()
    }

    fn headers(&self) -> Vec<(String, HeaderValue)> {
        self.oheaders
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn set_header_value(&mut self, header: &str, value: &str) -> prr::Result<()> {
        let header_to_set = HeaderName::from_str(header).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::HeaderName,
                value: Some(header.to_string()),
            })
        })?;
        let header_to_set_value = HeaderValue::from_str(value).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::HeaderValue,
                value: Some(value.to_string()),
            })
        })?;

        self.oheaders.insert(header_to_set, header_to_set_value);
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> prr::Result<()> {
        let header_to_remove = HeaderName::from_str(header).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::HeaderName,
                value: Some(header.to_string()),
            })
        })?;
        self.oheaders.remove(header_to_remove);
        Ok(())
    }

    fn add_header(&mut self, header: &str, value: &str) -> prr::Result<()> {
        self.set_header_value(header, value)
    }

    fn url(&self) -> prr::Result<url::Url> {
        Ok(self.ourl.clone())
    }

    fn set_url(&mut self, url: &url::Url) -> prr::Result<()> {
        self.ourl = url.clone();
        Ok(())
    }

    fn set_status(&mut self, response: &u16) -> prr::Result<()> {
        self.ostatus = Some(StatusCode::from_u16(*response).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::Status,
                value: Some(response.to_string()),
            })
        })?);
        Ok(())
    }

    fn clear_headers(&mut self) -> prr::Result<()> {
        self.oheaders.clear();
        Ok(())
    }

    fn set_body(&mut self, body: &Vec<u8>) -> prr::Result<()> {
        self.obody = body.clone();
        Ok(())
    }

    fn get_body(&self) -> prr::Result<&Vec<u8>> {
        Ok(&self.obody)
    }

    fn get_status(&self) -> prr::Result<StatusCode> {
        self.ostatus
            .ok_or(prr::Error::MissingValue(prr::ValueType::Status))
    }

    fn tpe(&self) -> prr::PrrType {
        self.tpe.clone()
    }

    fn set_method(&mut self, method: &http::Method) -> prr::Result<()> {
        self.omethod = method.clone();
        Ok(())
    }

    fn get_method(&self) -> http::Method {
        self.omethod.clone()
    }
}
