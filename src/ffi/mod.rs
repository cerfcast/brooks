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

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unnecessary_transmutes)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{
    ffi::{CStr, c_char},
    fs::OpenOptions,
    io::Read,
    marker::PhantomData,
    str::FromStr,
};

use http::{HeaderName, HeaderValue, Method, Request, Uri};
use libc::malloc;

use crate::{
    ffi::NginxTransformError::{BadHeaderName, BadHeaderValue, BadMethodValue, BadUri},
    mel::{
        scope::{Scopes, minimal_core_variable_types},
        tvs::Type,
    },
    ps::{
        interpret::{
            ProcessableRequestResponse,
            ProcessableRequestResponseError::{BadValue, InvalidMode},
            ProcessableRequestResponseResult, PsInterpretMode, interpret_stage,
        },
        spec::{TypedGenericStage, TypedStage},
        verify::{PsVerificationKey, verify_ps_request_stage},
    },
};

unsafe fn from_nginx_str(s: ngx_str_t) -> String {
    let mut v = vec![0u8; s.len];

    let mut d = s.data;
    for y in v.iter_mut().take(s.len) {
        *y = *d;
        d = d.wrapping_add(1);
    }

    String::from_utf8_unchecked(v)
}

unsafe fn to_nginx_str(s: &str) -> ngx_str_t {
    let len = s.len();
    let data = malloc(len) as *mut u8;
    let mut datai = data;

    for y in s.bytes() {
        *datai = y;
        datai = datai.wrapping_add(1);
    }

    ngx_str_t { len, data }
}

#[repr(C)]
pub struct BrooksC {
    _data: TypedStage<PsVerificationKey>,
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_analyze(
    path: *const c_char,
    cookie: *mut *const BrooksC,
) -> bool {
    let path = match CStr::from_ptr(path).to_str() {
        Ok(o) => o,
        Err(_) => return false,
    }
    .to_string();

    let mut ps_contents: Vec<u8> = vec![];
    let mut ps_file = match OpenOptions::new().read(true).open(path.clone()) {
        Ok(o) => o,
        Err(_) => {
            println!("Could not open path: {}", path);
            return false;
        }
    };

    if ps_file.read_to_end(&mut ps_contents).is_err() {
        return false;
    }

    let ps_source = &String::from_utf8_lossy(&ps_contents);

    let result = match serde_json::from_str::<TypedGenericStage>(ps_source) {
        Ok(o) => o,
        Err(_) => return false,
    };

    let types_scope = Scopes::<Type> {
        scopes: vec![minimal_core_variable_types()],
    };
    let result = match verify_ps_request_stage(&result, types_scope) {
        Ok(o) => o,
        Err(_) => return false,
    };

    *cookie = Box::into_raw(Box::new(BrooksC {
        _data: result,
        _marker: PhantomData {},
    }));

    true
}

#[derive(Debug, Clone)]
pub enum NginxTransformError {
    BadHeaderName(String),
    BadHeaderValue(String),
    BadMethodValue(String),
    BadUri(String),
    CreationError(String),
}

impl TryFrom<ngx_http_request_s> for Request<String> {
    type Error = NginxTransformError;

    fn try_from(value: ngx_http_request_s) -> Result<Self, Self::Error> {
        let mut header_part = &value.headers_in.headers.part;
        let mut header_element = header_part.elts as *mut ngx_table_elt_s;

        let mut request = Request::builder();
        unsafe {
            let mut i = 0usize;
            loop {
                if i >= header_part.nelts {
                    if header_part.next.is_null() {
                        break;
                    }

                    header_part = &(*header_part.next);
                    header_element = header_part.elts as *mut ngx_table_elt_s;
                    i = 0;
                }

                let k = from_nginx_str((*header_element).key);
                let val = from_nginx_str((*header_element).value);

                request = request.header(
                    HeaderName::from_str(&k).map_err(|_| BadHeaderName(k))?,
                    HeaderValue::from_str(&val).map_err(|_| BadHeaderValue(val))?,
                );

                header_element = header_element.wrapping_add(1);
                i += 1;
            }

            let parsed_uri = Uri::from_str(&format!(
                "{}?{}",
                from_nginx_str(value.uri),
                from_nginx_str(value.args)
            ))
            .map_err(|e| BadUri(e.to_string()))?;

            request = request.uri(parsed_uri.clone());

            request = request.method(
                Method::from_str(&from_nginx_str(value.method_name))
                    .map_err(|e| BadMethodValue(e.to_string()))?,
            );
        }

        request
            .body("".to_string())
            .map_err(|e| NginxTransformError::CreationError(e.to_string()))
    }
}

#[derive(Debug, Clone)]
struct ProcessedRequest {
    req: Request<String>,
    updated_uri: Option<Uri>,
}

impl ProcessableRequestResponse for ProcessedRequest {
    fn header_value(&self) -> Option<String> {
        todo!()
    }

    fn headers(&self) -> Vec<String> {
        self.req.headers().keys().map(|hv| hv.to_string()).collect()
    }

    fn set_header_value(
        &mut self,
        header: &str,
        value: &str,
    ) -> ProcessableRequestResponseResult<()> {
        self.req.headers_mut().insert(
            HeaderName::from_str(header).map_err(|_| BadValue)?,
            HeaderValue::from_str(value).map_err(|_| BadValue)?,
        );
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> ProcessableRequestResponseResult<()> {
        self.req
            .headers_mut()
            .remove(HeaderName::from_str(header).map_err(|_| BadValue)?);
        Ok(())
    }

    fn add_header(&mut self, header: &str, value: &str) -> ProcessableRequestResponseResult<()> {
        self.set_header_value(header, value)
    }

    fn uri(&self) -> Uri {
        self.req.uri().clone()
    }

    fn set_uri(&mut self, uri: &Uri) -> ProcessableRequestResponseResult<()> {
        self.updated_uri = Some(uri.clone());
        Ok(())
    }

    fn set_response(&mut self, _response: &u16) -> ProcessableRequestResponseResult<()> {
        Err(InvalidMode)
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_proxy(cookie: *mut BrooksC, req: *mut ngx_http_request_s) {
    let typed_stage = &(*cookie)._data;

    let http_req = match TryInto::<Request<String>>::try_into(*req) {
        Err(_) => {
            (*req).headers_out.status = 500;
            return;
        }
        Ok(o) => o,
    };

    let mut processed_http_req = ProcessedRequest {
        req: http_req,
        updated_uri: None,
    };

    let _result = interpret_stage(
        typed_stage,
        &mut processed_http_req,
        PsInterpretMode::Response,
    )
    .expect("Could not interpret a valid client response");

    todo!()
}
