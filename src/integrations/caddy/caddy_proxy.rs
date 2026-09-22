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
    marker::PhantomData,
};

use http::{StatusCode, header::HOST};
use libc::intptr_t;
use tokio::runtime;

use crate::{
    cdni::md::interpret::{HmdTransformError, MdInterpretError},
    integrations::{
        caddy::{
            GoInt, caddy_response_set_body, caddy_response_set_header, caddy_response_set_status,
            caddyi::{BrooksCaddyConfiguration, BrooksCaddyRequest, drain_to_caddy_log},
        },
        common::safe_brooks_integration_handle,
        hmds::{HmdsConfiguration, HmdsServerConfiguration},
        support::to_null_terminated_str,
    },
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::interpreter::builtins::builtin_builtin_function_interpreters,
};

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn caddy_brooks_configure(
    cookie: *mut *const BrooksCaddyConfiguration,
    config_str: *const c_char,
    caddy_log_cb: *mut c_void,
) -> bool {
    let mut log = LogMsgs::new_with_prefix("brooks configure", crate::logging::LogLevel::Debug);

    let config_str = match CStr::from_ptr(config_str).to_str() {
        Ok(o) => o,
        Err(e) => {
            log = error!(
                log,
                &format!("Could not convert given path into Rust string: {e}")
            );
            drain_to_caddy_log(caddy_log_cb, &log);
            return false;
        }
    };

    let server_config = match HmdsServerConfiguration::new_by_sense(config_str) {
        Ok(o) => o,
        Err(e) => {
            log = error!(
                log,
                &format!(
                    "Could not determine HMDS configuration from given string ({config_str}: {e}"
                )
            );
            drain_to_caddy_log(caddy_log_cb, &log);
            return false;
        }
    };

    *cookie = Box::into_raw(Box::new(BrooksCaddyConfiguration {
        hmds: HmdsConfiguration {
            hmds_server: server_config,
            hmds_cache: Default::default(),
        },
        _marker: PhantomData {},
    }));

    true
}

fn try_from_response(
    response: &http::Response<Vec<u8>>,
    status: StatusCode,
    reqres: *mut c_void,
) -> Result<(), Box<MdInterpretError>> {
    unsafe {
        for header in response.headers().iter() {
            let header_name = header.0.to_string();
            let header_value = header.1.to_str().map_err(|e| {
                MdInterpretError::TransformError(
                    HmdTransformError::BadHeaderValue(e.to_string()).into(),
                )
            })?;

            caddy_response_set_header(
                reqres,
                to_null_terminated_str(&header_name).as_ptr() as *const i8,
                to_null_terminated_str(header_value).as_ptr() as *const i8,
            );
        }

        caddy_response_set_status(reqres, status.as_u16() as i32);
    }

    Ok(())
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_caddy_proxy(
    cookie: *mut BrooksCaddyConfiguration,
    req: *mut c_void,
    res: *mut c_void,
    caddy_log_cb: *mut c_void,
) -> intptr_t {
    let log = LogMsgs::new_with_prefix("brooks proxy", crate::logging::LogLevel::Debug);

    let (result, log) = match do_brooks_caddy_proxy(cookie, req, res, log) {
        Err((e, mut log)) => {
            log = error!(
                log,
                &format!(
                    "Error occurred proxying original request according to configured host metadata: {e}",
                )
            );
            (-1, log)
        }
        Ok(mut log) => {
            log = debug!(
                log,
                "Successfully proxied original request according to configured host metadata"
            );
            (0, log)
        }
    };

    drain_to_caddy_log(caddy_log_cb, &log);
    result
}

unsafe fn do_brooks_caddy_proxy(
    cookie: *mut BrooksCaddyConfiguration,
    req: *mut c_void,
    res: *mut c_void,
    log: LogMsgs,
) -> Result<LogMsgs, (Box<MdInterpretError>, LogMsgs)> {
    // When interpreting MEL expressions in the HMD, use all builtin functions.
    let mel_scope = builtin_builtin_function_interpreters();

    let http_req = Box::from_raw(req as *mut BrooksCaddyRequest).request;

    let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
        Ok(o) => o,
        Err(e) => {
            return Err((MdInterpretError::RuntimeError(e.to_string()).into(), log));
        }
    };

    let hmds_key = match http_req.headers().get(HOST).cloned() {
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

    let (status, response, log) = safe_brooks_integration_handle(
        &http_req,
        &Some(mel_scope),
        hmds_key,
        &mut (*cookie).hmds,
        &runtime,
        log,
    )?;

    if let Err(e) = try_from_response(&response, status, res) {
        return Err((e, log));
    }

    caddy_response_set_body(
        res,
        response.body().len() as GoInt,
        response.body().as_ptr(),
    );

    Ok(log)
}
