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

use std::{collections::HashMap, marker::PhantomData, path::PathBuf, ptr::null, str::FromStr};

use http::{HeaderName, HeaderValue, Method, Request, StatusCode, Uri};
use libc::intptr_t;
use reqwest::Response;
use tokio::runtime;

use crate::{
    integrations::{
        common::{
            BrooksIntegrationTransformError, BrooksIntegrationsProxyError,
            safe_brooks_integration_handle,
        },
        hmds::HmdsConfiguration,
        nginx::{
            nginx_lib::{from_nginx_str, log_nginx_msgs, to_nginx_buf, to_nginx_str},
            ngx_buf_s, ngx_http_request_s, ngx_list_push, ngx_log_s, ngx_str_t, ngx_table_elt_s,
        },
    },
    logging::{LogLevel, LogMsg, LogMsgs},
};

#[repr(C)]
pub struct NginxBrooksConfiguration {
    hmds: HmdsConfiguration,
    _marker: core::marker::PhantomData<*mut u8>,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_configure(
    cookie: *mut *const NginxBrooksConfiguration,
    hmds_path: ngx_str_t,
    nlx: *mut ngx_log_s,
) -> bool {
    let log = LogMsgs::new_with_prefix("brooks analysis", crate::logging::LogLevel::Debug);

    *cookie = Box::into_raw(Box::new(NginxBrooksConfiguration {
        hmds: HmdsConfiguration {
            hmds_path: PathBuf::from(from_nginx_str(hmds_path)),
            hmds_cache: HashMap::new(),
        },
        _marker: PhantomData {},
    }));

    log_nginx_msgs(nlx, &log);

    true
}

impl TryFrom<ngx_http_request_s> for Request<String> {
    type Error = Box<BrooksIntegrationTransformError>;

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
                    HeaderName::from_str(&k)
                        .map_err(|_| BrooksIntegrationTransformError::BadHeaderName(k))?,
                    HeaderValue::from_str(&val)
                        .map_err(|_| BrooksIntegrationTransformError::BadHeaderValue(val))?,
                );

                header_element = header_element.wrapping_add(1);
                i += 1;
            }

            let parsed_uri = Uri::from_str(&format!(
                "{}?{}",
                from_nginx_str(value.uri),
                from_nginx_str(value.args)
            ))
            .map_err(|e| BrooksIntegrationTransformError::BadUri(e.to_string()))?;

            request = request.uri(parsed_uri.clone());

            request = request.method(
                Method::from_str(&from_nginx_str(value.method_name))
                    .map_err(|e| BrooksIntegrationTransformError::BadMethodValue(e.to_string()))?,
            );
        }

        request
            .body("".to_string())
            .map_err(|e| Box::new(BrooksIntegrationTransformError::BadBody(e.to_string())))
    }
}

unsafe fn try_from_response(
    response: &Response,
    status: StatusCode,
    req: *mut ngx_http_request_s,
) -> Result<(), Box<BrooksIntegrationTransformError>> {
    for (hsh, header) in response.headers().iter().enumerate() {
        let header_name = header.0.to_string();
        let header_value = header
            .1
            .to_str()
            .map_err(|e| BrooksIntegrationTransformError::BadHeaderValue(e.to_string()))?;

        let he = ngx_list_push(&mut (*req).headers_out.headers) as *mut ngx_table_elt_s;

        (*he).hash = hsh;
        (*he).key = to_nginx_str(&header_name, (*req).pool);
        (*he).value = to_nginx_str(header_value, (*req).pool);
    }

    (*req).headers_out.status = status.as_u16() as usize;

    Ok(())
}

enum NginxReturnCodes {
    Ok = 0,
    Error = -1,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_proxy(
    cookie: *mut NginxBrooksConfiguration,
    req: *mut ngx_http_request_s,
    body: *mut *mut ngx_buf_s,
) -> intptr_t {
    let mut log = LogMsgs::new_with_prefix("brooks proxy", crate::logging::LogLevel::Debug);

    let mut result = NginxReturnCodes::Ok;
    match do_ngx_brooks_proxy(cookie, req, body, &mut log) {
        Ok(_) => {
            log = error!(log, "Successful proxy");
        }
        Err(e) => {
            log = error!(log, &e.to_string());
            (*req).headers_out.content_length_n = e.to_string().len() as i64;
            (*req).headers_out.status = 500;
            *body = match to_nginx_buf(&e.to_string().into_bytes(), (*req).pool) {
                Ok(o) => o,
                Err(e) => {
                    log = error!(
                        log,
                        &format!("Failed to generate body of proxy response: {}", e)
                    );
                    // The only error that we want nginx to handle is the one where
                    // we cannot generate a body. All other errors will generate valid
                    // HTTP responses (even if those HTTP responses are, themselves, errors).
                    result = NginxReturnCodes::Error;
                    null::<*mut ngx_buf_s>() as *mut ngx_buf_s
                }
            };
        }
    }
    log_nginx_msgs((*(*req).connection).log, &log);
    result as intptr_t
}

unsafe fn do_ngx_brooks_proxy(
    cookie: *mut NginxBrooksConfiguration,
    req: *mut ngx_http_request_s,
    body: *mut *mut ngx_buf_s,
    log: &mut LogMsgs,
) -> Result<(), Box<BrooksIntegrationsProxyError>> {
    let mut http_req = TryInto::<Request<String>>::try_into(*req)
        .map_err(|e| Box::new(BrooksIntegrationsProxyError::TransformError(e)))?;

    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| BrooksIntegrationsProxyError::RuntimeError(e.to_string()))?;

    let (status, response) =
        safe_brooks_integration_handle(&mut http_req, &mut (*cookie).hmds, &runtime, log)?;

    try_from_response(&response, status, req)
        .map_err(BrooksIntegrationsProxyError::TransformError)?;

    let result_body = runtime
        .block_on(response.bytes())
        .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

    *body = to_nginx_buf(&result_body, (*req).pool)
        .map_err(BrooksIntegrationsProxyError::TransformError)?;

    // Indicate that the response should use chunked encoding.
    (*req).headers_out.content_length_n = -1;

    Ok(())
}
