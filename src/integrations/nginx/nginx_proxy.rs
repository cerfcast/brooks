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

use std::{collections::HashMap, marker::PhantomData, ptr::null, str::FromStr};

use http::{HeaderName, HeaderValue, Method, Request, StatusCode, Uri, header::HOST};
use libc::intptr_t;
use reqwest::Response;
use tokio::runtime;

use crate::{
    cdni::md::interpret::{HmdTransformError, MdInterpretError},
    environment::scope::Scopes,
    integrations::{
        common::safe_brooks_integration_handle,
        hmds::{HmdsConfiguration, HmdsServerConfiguration},
        nginx::{
            nginx_lib::{from_nginx_str, log_nginx_msgs, to_nginx_buf, to_nginx_str},
            ngx_buf_s, ngx_http_request_s, ngx_list_push, ngx_log_s, ngx_str_t, ngx_table_elt_s,
        },
    },
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::interpreter::{builtins::builtin_builtin_function_interpreters, interpret::TypedValue},
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
    config_str: ngx_str_t,
    nlx: *mut ngx_log_s,
) -> bool {
    let mut log = LogMsgs::new_with_prefix("brooks analysis", crate::logging::LogLevel::Debug);

    let config_str = from_nginx_str(config_str);
    let server_config = match HmdsServerConfiguration::new_by_sense(&config_str) {
        Ok(o) => o,
        Err(e) => {
            log = error!(
                log,
                &format!(
                    "Could not determine HMDS configuration from given string ({config_str}: {e}"
                )
            );
            log_nginx_msgs(nlx, &log);
            return false;
        }
    };

    *cookie = Box::into_raw(Box::new(NginxBrooksConfiguration {
        hmds: HmdsConfiguration {
            hmds_server: server_config,
            hmds_cache: HashMap::new(),
        },
        _marker: PhantomData {},
    }));

    log_nginx_msgs(nlx, &log);

    true
}

impl TryFrom<ngx_http_request_s> for Request<Vec<u8>> {
    type Error = Box<HmdTransformError>;

    fn try_from(value: ngx_http_request_s) -> Result<Self, Self::Error> {
        let mut header_part = &value.headers_in.headers.part;
        let mut header_element = header_part.elts as *mut ngx_table_elt_s;

        let mut request = Request::builder();
        unsafe {
            let mut host: Option<String> = None;

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

                let header_name =
                    HeaderName::from_str(&k).map_err(|_| HmdTransformError::BadHeaderName(k))?;
                let header_value = HeaderValue::from_str(&val)
                    .map_err(|_| HmdTransformError::BadHeaderValue(val.clone()))?;
                request = request.header(header_name.clone(), header_value.clone());

                if header_name == HOST {
                    host = Some(val);
                }

                header_element = header_element.wrapping_add(1);
                i += 1;
            }

            let host = match host {
                Some(h) => h,
                None => return Err(HmdTransformError::BadUrl(from_nginx_str(value.uri)).into()),
            };

            let http_s = if (*value.http_connection).ssl() != 0 {
                "https".to_string()
            } else {
                "http".to_string()
            };

            let parsed_uri = Uri::from_str(&format!(
                "{}://{}{}?{}",
                http_s,
                host,
                from_nginx_str(value.uri),
                from_nginx_str(value.args)
            ))
            .map_err(|e| HmdTransformError::BadUri(e.to_string()))?;

            request = request.uri(parsed_uri.clone());

            request = request.method(
                Method::from_str(&from_nginx_str(value.method_name))
                    .map_err(|e| HmdTransformError::BadMethodValue(e.to_string()))?,
            );
        }

        request
            .body(vec![])
            .map_err(|e| Box::new(HmdTransformError::BadBody(e.to_string())))
    }
}

unsafe fn try_from_response(
    response: &Response,
    status: StatusCode,
    req: *mut ngx_http_request_s,
) -> Result<(), Box<HmdTransformError>> {
    for (hsh, header) in response.headers().iter().enumerate() {
        let header_name = header.0.to_string();
        let header_value = header
            .1
            .to_str()
            .map_err(|e| HmdTransformError::BadHeaderValue(e.to_string()))?;

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
    let log = LogMsgs::new_with_prefix("brooks proxy", crate::logging::LogLevel::Debug);

    let mut result = NginxReturnCodes::Ok;
    let log = match do_ngx_brooks_proxy(cookie, req, body, log) {
        Ok(mut log) => {
            log = error!(log, "Successful proxy");
            log
        }
        Err((e, mut log)) => {
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
            log
        }
    };
    log_nginx_msgs((*(*req).connection).log, &log);
    result as intptr_t
}

unsafe fn do_ngx_brooks_proxy(
    cookie: *mut NginxBrooksConfiguration,
    req: *mut ngx_http_request_s,
    body: *mut *mut ngx_buf_s,
    log: LogMsgs,
) -> Result<LogMsgs, (Box<MdInterpretError>, LogMsgs)> {
    // When interpreting MEL expressions in the HMD, use all builtin functions.
    let mel_scope: Scopes<TypedValue> = builtin_builtin_function_interpreters().into();

    let http_req = match TryInto::<Request<Vec<u8>>>::try_into(*req) {
        Ok(o) => o,
        Err(e) => return Err((MdInterpretError::TransformError(e).into(), log)),
    };

    let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
        Ok(o) => o,
        Err(e) => {
            return Err((MdInterpretError::RuntimeError(e.to_string()).into(), log));
        }
    };

    let hmds_key = match http_req.headers().get(HOST) {
        Some(o) => o,
        None => {
            return Err((
                MdInterpretError::ProxyError("Could not get host from request".to_string()).into(),
                log,
            ));
        }
    };
    let hmds_key = match hmds_key.to_str() {
        Ok(o) => o,
        Err(e) => {
            return Err((MdInterpretError::ProxyError(e.to_string()).into(), log));
        }
    };

    let (_status, response, log) = safe_brooks_integration_handle(
        &http_req,
        mel_scope,
        hmds_key,
        &mut (*cookie).hmds,
        &runtime,
        log,
    )?;

    *body = match to_nginx_buf(response.body(), (*req).pool) {
        Ok(o) => o,
        Err(e) => return Err((MdInterpretError::TransformError(e).into(), log)),
    };

    // Indicate that the response should use chunked encoding.
    (*req).headers_out.content_length_n = -1;
    (*req).headers_out.status = _status.as_u16() as usize;

    Ok(log)
}
