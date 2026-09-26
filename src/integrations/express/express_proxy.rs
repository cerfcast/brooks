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

use http::header::HOST;
use tokio::runtime;

use crate::{
    cdni::md::interpret::MdInterpretError,
    environment::scope::Scopes,
    integrations::{
        common::safe_brooks_integration_handle,
        express::expressi::{BrooksExpressConfiguration, BrooksExpressRequest},
        hmds::{HmdsConfiguration, HmdsServerConfiguration},
        support::to_null_terminated_str,
    },
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::interpreter::{builtins::builtin_builtin_function_interpreters, interpret::TypedValue},
};

pub type BrooksExpressLoggerCallback = unsafe extern "C" fn(u8, *const c_char);

pub(crate) enum BrooksExpressLogLevel {
    Trace = 0,
    Debug,
    Warn,
    Error,
}

impl From<LogLevel> for BrooksExpressLogLevel {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => BrooksExpressLogLevel::Trace,
            LogLevel::Debug => BrooksExpressLogLevel::Debug,
            LogLevel::Warn => BrooksExpressLogLevel::Warn,
            LogLevel::Error => BrooksExpressLogLevel::Error,
        }
    }
}
unsafe fn drain_to_express_log(express_log: BrooksExpressLoggerCallback, log: &LogMsgs) {
    for msg in log.use_msgs() {
        unsafe {
            express_log(
                Into::<BrooksExpressLogLevel>::into(msg.level()) as u8,
                to_null_terminated_str(&msg.msg()).as_ptr() as *const i8,
            );
        }
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn express_brooks_configure(
    config_str: *mut c_void,
    result: *mut *mut c_void,
    express_log_cb: BrooksExpressLoggerCallback,
) -> bool {
    let mut log = LogMsgs::new_with_prefix("brooks configure", crate::logging::LogLevel::Debug);

    let config_str = config_str as *mut i8;

    let config_str = match CStr::from_ptr(config_str).to_str() {
        Ok(o) => o,
        Err(e) => {
            log = error!(
                log,
                &format!("Could not convert given path into Rust string: {e}")
            );
            drain_to_express_log(express_log_cb, &log);
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
            drain_to_express_log(express_log_cb, &log);
            return false;
        }
    };

    *result = Box::into_raw(Box::new(BrooksExpressConfiguration {
        hmds: HmdsConfiguration {
            hmds_server: server_config,
            hmds_cache: Default::default(),
        },
        _marker: PhantomData {},
    })) as *mut c_void;

    true
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brooks_express_proxy(
    cookie: *mut BrooksExpressConfiguration,
    req: *mut c_void,
    res: *mut c_void,
    express_log_cb: BrooksExpressLoggerCallback,
) -> usize {
    let log = LogMsgs::new_with_prefix("brooks proxy", crate::logging::LogLevel::Debug);

    let (result, log) = match do_brooks_express_proxy(cookie, req, res, log) {
        Err((e, mut log)) => {
            log = error!(
                log,
                &format!(
                    "Error occurred proxying original request according to configured host metadata: {e}",
                )
            );
            (0, log)
        }
        Ok(mut log) => {
            log = debug!(
                log,
                "Successfully proxied original request according to configured host metadata"
            );
            (0, log)
        }
    };

    drain_to_express_log(express_log_cb, &log);
    result
}

unsafe fn do_brooks_express_proxy(
    cookie: *mut BrooksExpressConfiguration,
    req: *mut c_void,
    res: *mut c_void,
    log: LogMsgs,
) -> Result<LogMsgs, (Box<MdInterpretError>, LogMsgs)> {
    // When interpreting MEL expressions in the HMD, use all builtin functions.
    let mel_scope: Scopes<TypedValue> = builtin_builtin_function_interpreters().into();

    let http_req = Box::from_raw(req as *mut BrooksExpressRequest).request;

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
        mel_scope,
        hmds_key,
        &mut (*cookie).hmds,
        &runtime,
        log,
    )?;

    /*
    if let Err(e) = try_from_response(&response, status, res) {
        return Err((e, log));
    }

    caddy_response_set_body(
        res,
        response.body().len() as GoInt,
        response.body().as_ptr(),
    );

    */
    Ok(log)
}
