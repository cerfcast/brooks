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

impl std::error::Error for NativeRequestTransformError {}
