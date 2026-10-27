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
    str::FromStr,
};

use crate::{
    processing::processors::{SimpleProcessorsInterpreterContext, ps::PsInterpretValue},
    prr::{self, Prr, PrrType, ValueType},
    utils::try_to_url,
    verifying::verify::CdniVerificationKey,
};

/// Concrete instances of MI generated during interpretation that may be used to specify post-interpretation behavior.
pub trait MetadataInformationResultElement: Debug {}

/// TODO
#[derive(Debug, Default)]
pub struct MetadataInformationResultElements {
    pub elements: Vec<Box<dyn MetadataInformationResultElement>>,
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

impl std::error::Error for Error {}

/// The context for the interpretation of a validated representation of a given piece of Metadata Information.
///
///
/// The `cookie` (of user-specified type `C`) can be used to allow access to other data
/// an interpretation of the MI may update.
///
/// TODO: Determine whether the runtime can be factored out (made generic).
#[derive(Debug)]
pub struct InterpreterContext<C> {
    pub request: Option<http::Request<Vec<u8>>>,
    pub response: Option<reqwest::Response>,
    pub runtime: tokio::runtime::Runtime,
    pub cookie: C,
}

/// TODO
pub type InterpretationResult<Ck, Res, Error> = Result<(Ck, Res), Box<Error>>;

/// Interpret a validated representation of a given piece of Metadata Information.
pub trait Interpreter<Ck, Res, Error>: Debug {
    /// Perform the semantics of the MI.
    fn interpret(&self, input: Ck) -> InterpretationResult<Ck, Res, Error>;

    fn at(&self, stage: Stages) -> bool;
}

/// TODO
#[derive(Debug)]
pub struct ProcessedRequestResponse {
    pub oheaders: http::HeaderMap<http::HeaderValue>,
    pub obody: Vec<u8>,
    pub ourl: url::Url,
    pub ostatus: Option<http::StatusCode>,
    pub omethod: http::Method,
    pub tpe: prr::PrrType,
}

impl ProcessedRequestResponse {
    pub fn new_request(url: &url::Url, method: &http::Method) -> Self {
        ProcessedRequestResponse {
            oheaders: http::HeaderMap::<http::HeaderValue>::default(),
            obody: vec![],
            ourl: url.clone(),
            ostatus: None,
            tpe: prr::PrrType::Request,
            omethod: method.clone(),
        }
    }

    pub fn new_response(url: &url::Url, method: &http::Method) -> Self {
        ProcessedRequestResponse {
            oheaders: http::HeaderMap::<http::HeaderValue>::default(),
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

        let mut headers = http::HeaderMap::<http::HeaderValue>::default();
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
    fn headers(&self) -> Vec<(String, http::HeaderValue)> {
        self.oheaders
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn set_header_value(&mut self, header: &str, value: &str) -> prr::Result<()> {
        let header_to_set = http::HeaderName::from_str(header).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::HeaderName,
                value: Some(header.to_string()),
            })
        })?;
        let header_to_set_value = http::HeaderValue::from_str(value).map_err(|_| {
            prr::Error::BadValue(prr::Value {
                tpe: prr::ValueType::HeaderValue,
                value: Some(value.to_string()),
            })
        })?;

        self.oheaders.insert(header_to_set, header_to_set_value);
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> prr::Result<()> {
        let header_to_remove = http::HeaderName::from_str(header).map_err(|_| {
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
        self.ostatus = Some(http::StatusCode::from_u16(*response).map_err(|_| {
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

    fn get_status(&self) -> prr::Result<http::StatusCode> {
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

impl TryFrom<&dyn Prr<Vec<u8>>> for reqwest::Request {
    type Error = prr::Error;

    fn try_from(value: &dyn Prr<Vec<u8>>) -> std::result::Result<Self, Self::Error> {
        if value.tpe() != PrrType::Request {
            return Err(prr::Error::InvalidMode);
        }

        let url = value
            .url()
            .map_err(|_| prr::Error::MissingValue(ValueType::Url))?;

        let mut request = reqwest::Request::new(reqwest::Method::GET, url);

        // Put in the headers.
        for (header_key, header_value) in value.headers() {
            request.headers_mut().insert(
                http::HeaderName::from_str(&header_key).map_err(|_| {
                    prr::Error::BadValue(prr::Value {
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
                .map_err(|_| prr::Error::MissingValue(ValueType::Body))?
                .clone()
                .into(),
        );

        Ok(request)
    }
}

// Generic implementation of `interpret::Interpreter` for CdniVerificationKey
// that simply redirects to the implementation of the underlying key.
impl
    Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > for CdniVerificationKey
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > {
        match &self {
            CdniVerificationKey::Stage(typed_stage) => typed_stage.interpret(input),
            CdniVerificationKey::Source(typed_source) => typed_source.interpret(input),
            CdniVerificationKey::SetVariable(typed_set_variable) => {
                typed_set_variable.interpret(input)
            }
            CdniVerificationKey::None => todo!(),
        }
    }

    fn at(&self, stage: Stages) -> bool {
        match &self {
            CdniVerificationKey::Stage(typed_stage) => typed_stage.at(stage),
            CdniVerificationKey::Source(typed_source) => typed_source.at(stage),
            CdniVerificationKey::SetVariable(typed_set_variable) => typed_set_variable.at(stage),
            CdniVerificationKey::None => todo!(),
        }
    }
}
