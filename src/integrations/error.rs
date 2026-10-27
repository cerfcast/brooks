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

use std::fmt::Display;

use http::StatusCode;

use crate::{logging::LogMsgs, processing::error::ImdInterpretError};

pub type ProxyValue = (StatusCode, http::Response<Vec<u8>>, LogMsgs);
pub type ProxyResult = Result<ProxyValue, (Box<ProxyError>, LogMsgs)>;

#[derive(Debug, Clone)]
pub enum NativeRequestTransformError {
    BadHeaderName(String),
    BadHeaderValue(String),
    BadMethodValue(String),
    BadBody(String),
    BadUri(String),
    BadUrl(String),
    CreationError(String),
    BadMemory,
}

impl Display for NativeRequestTransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error manipulating the native request: ")?;
        match self {
            NativeRequestTransformError::BadHeaderName(bhn) => {
                write!(f, "Bad header name: {bhn}")
            }
            NativeRequestTransformError::BadHeaderValue(bhv) => {
                write!(f, "Bad header value: {bhv}")
            }
            NativeRequestTransformError::BadMethodValue(bmv) => {
                write!(f, "Bad method value: {bmv}")
            }
            NativeRequestTransformError::BadBody(bb) => write!(f, "Bad body: {bb}"),
            NativeRequestTransformError::BadUri(bu) => write!(f, "Bad URI: {bu}"),
            NativeRequestTransformError::BadUrl(bu) => write!(f, "Bad URL: {bu}"),
            NativeRequestTransformError::CreationError(ce) => write!(f, "Creation error: {ce}"),
            NativeRequestTransformError::BadMemory => {
                write!(f, "Memory error")
            }
        }
    }
}

#[derive(Debug)]
pub enum ProxyError {
    SetupError(Box<dyn std::error::Error>),
    HmdsError(Box<dyn std::error::Error>),
    ImdiError(ImdInterpretError),
    NativeRequestTransformError(Box<NativeRequestTransformError>),
}

impl Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error proxying to/from the native request: ")?;
        match &self {
            ProxyError::SetupError(error) => write!(f, "{error}"),
            ProxyError::HmdsError(error) => write!(f, "{error}"),
            ProxyError::ImdiError(imd_interpret_error) => write!(f, "{imd_interpret_error}"),
            ProxyError::NativeRequestTransformError(native_request_transform_error) => {
                write!(f, "{native_request_transform_error}")
            }
        }
    }
}
