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
    ffi::{CStr, c_char, c_void},
    ptr::null,
    str::FromStr,
};

use http::{
    HeaderName, HeaderValue, Method, Request, Response, Uri, header::HOST,
    request::Builder as RequestBuilder, response::Builder as ResponseBuilder,
};

use crate::{
    integrations::{caddy::caddy_log, hmds::HmdsConfiguration, support::to_null_terminated_str},
    logging::{LogLevel, LogMsgs},
};

#[repr(C)]
pub struct BrooksCaddyConfiguration {
    pub(crate) hmds: HmdsConfiguration,
    pub(crate) _marker: core::marker::PhantomData<*mut u8>,
}

pub(crate) unsafe fn drain_to_caddy_log(cl: *mut c_void, log: &LogMsgs) {
    for msg in log.use_msgs() {
        let cmsg = to_null_terminated_str(&msg.msg());
        caddy_log(
            cl,
            Into::<BrooksCaddyLogLevel>::into(msg.level()) as u8,
            cmsg.as_ptr() as *const i8,
        );
    }
}

#[repr(u8)]
pub(crate) enum BrooksCaddyLogLevel {
    Trace = 0,
    Debug,
    Warn,
    Error,
}

impl From<LogLevel> for BrooksCaddyLogLevel {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => BrooksCaddyLogLevel::Trace,
            LogLevel::Debug => BrooksCaddyLogLevel::Debug,
            LogLevel::Warn => BrooksCaddyLogLevel::Warn,
            LogLevel::Error => BrooksCaddyLogLevel::Error,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BrooksCaddyRequestBuilder {
    builder: RequestBuilder,
}

pub(crate) struct BrooksCaddyRequest {
    pub(crate) request: Request<String>,
}

impl BrooksCaddyRequestBuilder {
    pub fn new() -> Self {
        BrooksCaddyRequestBuilder {
            builder: RequestBuilder::new(),
        }
    }

    pub fn set_header(self, hn: HeaderName, hv: HeaderValue) -> Self {
        BrooksCaddyRequestBuilder {
            builder: self.builder.header(hn, hv),
        }
    }

    pub fn set_uri(self, uri: Uri) -> Self {
        BrooksCaddyRequestBuilder {
            builder: self.builder.uri(uri),
        }
    }

    pub fn set_method(self, method: Method) -> Self {
        BrooksCaddyRequestBuilder {
            builder: self.builder.method(method),
        }
    }

    pub fn finalize_with_body(self, body: String) -> http::Result<BrooksCaddyRequest> {
        Ok(BrooksCaddyRequest {
            request: self.builder.body(body)?,
        })
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_new() -> *const c_void {
    Box::into_raw(Box::new(BrooksCaddyRequestBuilder::new())) as *const c_void
}

#[allow(unused_macros)]
macro_rules! ok_or_null {
    ($nameloc:expr) => {
        match $nameloc {
            Ok(o) => o,
            _ => return null(),
        }
    };
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_set_header(
    crb: *mut c_void,
    raw_hn: *const c_char,
    raw_hv: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyRequestBuilder);
    let maybe_header_name = CStr::from_ptr(raw_hn).to_string_lossy().into_owned();
    let maybe_header_value = CStr::from_ptr(raw_hv).to_string_lossy().into_owned();
    let header_name = ok_or_null!(HeaderName::try_from(maybe_header_name));
    let header_value = ok_or_null!(HeaderValue::try_from(maybe_header_value));
    Box::into_raw(Box::new(crb.set_header(header_name, header_value))) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_set_uri(
    crb: *mut c_void,
    raw_uri: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyRequestBuilder);
    let maybe_uri = CStr::from_ptr(raw_uri).to_string_lossy().into_owned();
    let uri = ok_or_null!(Uri::try_from(maybe_uri));
    Box::into_raw(Box::new(crb.set_uri(uri))) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_set_host(
    crb: *mut c_void,
    raw_host: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyRequestBuilder);
    let maybe_host = CStr::from_ptr(raw_host).to_string_lossy().into_owned();
    let host_header_value = ok_or_null!(HeaderValue::try_from(maybe_host));
    Box::into_raw(Box::new(crb.set_header(HOST, host_header_value))) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_set_method(
    crb: *mut c_void,
    raw_method: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyRequestBuilder);
    let maybe_method = CStr::from_ptr(raw_method).to_string_lossy().into_owned();
    let method = ok_or_null!(Method::from_str(&maybe_method));
    Box::into_raw(Box::new(crb.set_method(method))) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_request_builder_finalize_with_body(
    crb: *mut c_void,
    body: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyRequestBuilder);
    let body = if !body.is_null() {
        CStr::from_ptr(body).to_string_lossy().into_owned()
    } else {
        "".to_string()
    };
    Box::into_raw(Box::new(crb.finalize_with_body(body))) as *const c_void
}

#[derive(Debug)]
pub(crate) struct BrooksCaddyResponseBuilder {
    builder: ResponseBuilder,
}

pub(crate) struct BrooksCaddyResponse {
    pub(crate) response: Response<String>,
}

impl BrooksCaddyResponseBuilder {
    pub fn new() -> Self {
        BrooksCaddyResponseBuilder {
            builder: ResponseBuilder::new(),
        }
    }

    pub fn set_header(self, hn: HeaderName, hv: HeaderValue) -> Self {
        BrooksCaddyResponseBuilder {
            builder: self.builder.header(hn, hv),
        }
    }

    pub fn finalize_with_body(self, body: String) -> http::Result<BrooksCaddyResponse> {
        Ok(BrooksCaddyResponse {
            response: self.builder.body(body)?,
        })
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_response_builder_new() -> *const c_void {
    Box::into_raw(Box::new(BrooksCaddyResponseBuilder::new())) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_response_builder_set_header(
    crb: *mut c_void,
    raw_hn: *const c_char,
    raw_hv: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyResponseBuilder);
    let maybe_header_name = CStr::from_ptr(raw_hn).to_string_lossy().into_owned();
    let maybe_header_value = CStr::from_ptr(raw_hv).to_string_lossy().into_owned();
    let header_name = ok_or_null!(HeaderName::try_from(maybe_header_name));
    let header_value = ok_or_null!(HeaderValue::try_from(maybe_header_value));
    Box::into_raw(Box::new(crb.set_header(header_name, header_value))) as *const c_void
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_response_builder_finalize_with_body(
    crb: *mut c_void,
    body: *const c_char,
) -> *const c_void {
    let crb = Box::from_raw(crb as *mut BrooksCaddyResponseBuilder);
    let body = if !body.is_null() {
        CStr::from_ptr(body).to_string_lossy().into_owned()
    } else {
        "".to_string()
    };
    Box::into_raw(Box::new(crb.finalize_with_body(body))) as *const c_void
}
