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
    fs,
    marker::PhantomData,
    path::PathBuf,
};

use chrono::Utc;
use http::{StatusCode, header::HOST};
use libc::intptr_t;
use reqwest::Response;
use tokio::runtime;

use crate::{
    cdni::{spec::TypedHostMetadata, verify::verify_host_metadata},
    integrations::{
        caddy::{
            GoInt, caddy_response_set_body, caddy_response_set_header, caddy_response_set_status,
            caddyi::{BrooksCaddyConfiguration, BrooksCaddyRequest, drain_to_caddy_log},
        },
        common::{
            BrooksIntegrationTransformError, BrooksIntegrationsProxyError, ProcessedRequest,
            safe_brooks_integrations_proxy,
        },
        hmds::query_hmds,
        support::to_null_terminated_str,
    },
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{
        scope::{Scopes, minimal_core_variable_types},
        tvs::Type,
    },
};

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn caddy_brooks_configure(
    cookie: *mut *const BrooksCaddyConfiguration,
    path: *const c_char,
    caddy_log_cb: *mut c_void,
) -> bool {
    let mut log = LogMsgs::new_with_prefix("brooks configure", crate::logging::LogLevel::Debug);

    let path = match CStr::from_ptr(path).to_str() {
        Ok(o) => PathBuf::from(o),
        Err(e) => {
            log = error!(
                log,
                &format!("Could not convert given path into Rust string: {e}")
            );
            drain_to_caddy_log(caddy_log_cb, &log);
            return false;
        }
    };

    let exists = match fs::exists(&path) {
        Ok(o) => o,
        Err(e) => {
            log = error!(
                log,
                &format!("Could not check whether the given path {path:?} exists: {e}")
            );
            drain_to_caddy_log(caddy_log_cb, &log);
            return false;
        }
    };
    if !exists {
        log = error!(log, &format!("the given path {path:?} does not exist"));
        drain_to_caddy_log(caddy_log_cb, &log);
        return false;
    }

    *cookie = Box::into_raw(Box::new(BrooksCaddyConfiguration {
        hmds_path: path,
        hmds_cache: Default::default(),
        _marker: PhantomData {},
    }));

    true
}

fn try_from_response(
    response: &Response,
    status: StatusCode,
    reqres: *mut c_void,
) -> Result<(), Box<BrooksIntegrationTransformError>> {
    unsafe {
        for header in response.headers().iter() {
            let header_name = header.0.to_string();
            let header_value = header
                .1
                .to_str()
                .map_err(|e| BrooksIntegrationTransformError::BadHeaderValue(e.to_string()))?;

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
    let mut log = LogMsgs::new_with_prefix("brooks proxy", crate::logging::LogLevel::Debug);

    let result: intptr_t = if let Err(e) = do_brooks_caddy_proxy(cookie, req, res, &mut log) {
        log = error!(
            log,
            &format!(
                "Error occurred proxying original request according to configured host metadata: {e}",
            )
        );
        -1
    } else {
        log = debug!(
            log,
            "Successfully proxied original request according to configured host metadata"
        );
        0
    };

    drain_to_caddy_log(caddy_log_cb, &log);
    result
}

unsafe fn do_brooks_caddy_proxy(
    cookie: *mut BrooksCaddyConfiguration,
    req: *mut c_void,
    res: *mut c_void,
    log: &mut LogMsgs,
) -> Result<(), Box<BrooksIntegrationsProxyError>> {
    let mut http_req = Box::from_raw(req as *mut BrooksCaddyRequest).request;

    let host = http_req
        .headers()
        .get(HOST)
        .ok_or(BrooksIntegrationsProxyError::ProxyError(
            "Could not get host from request".to_string(),
        ))?
        .to_str()
        .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| BrooksIntegrationsProxyError::RuntimeError(e.to_string()))?;

    // First, try to find the query in the cache.
    *log = debug!(
        log,
        &format!("Looking for {host} in the host metadata cache.")
    );

    let found = match (*cookie).hmds_cache.get(host) {
        Some((timeout, found)) => {
            *log = debug!(
                log,
                &format!("Found {host} in the host metadata cache -- it is valid until {timeout}.")
            );
            if Utc::now() > *timeout {
                *log = debug!(
                    log,
                    &format!("{host} in the host metadata cache timed out.")
                );
                (*cookie).hmds_cache.remove(host);
                None
            } else {
                Some(found.clone())
            }
        }
        None => None,
    };

    let found = match found {
        Some(found) => found,
        None => {
            let (expiry, query_result) = match runtime
                .block_on(query_hmds(host, &(*cookie).hmds_path))
                .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?
            {
                Some((timeout, query_result)) => (timeout, query_result),
                None => {
                    return Err(BrooksIntegrationsProxyError::MissingConfiguration(
                        host.to_string(),
                    )
                    .into());
                }
            };

            let metadata = serde_json::from_value::<TypedHostMetadata<()>>(query_result)
                .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

            let types_scope = Scopes::<Type> {
                scopes: vec![minimal_core_variable_types()],
            };
            let found = verify_host_metadata(&metadata.value, types_scope)
                .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

            *log = debug!(
                log,
                &format!("Put {host} in the host metadata cache to expire at {expiry}.")
            );
            (*cookie)
                .hmds_cache
                .insert(host.to_string(), (expiry, found.clone()));

            found
        }
    };

    let mut processed_http_req = ProcessedRequest {
        req: &mut http_req,
        updated_uri: None,
    };

    let (status, response) =
        safe_brooks_integrations_proxy(&found, &mut processed_http_req, &runtime, log)?;

    try_from_response(&response, status, res)
        .map_err(BrooksIntegrationsProxyError::TransformError)?;

    let result_body = runtime
        .block_on(response.bytes())
        .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

    caddy_response_set_body(res, result_body.len() as GoInt, result_body.as_ptr());

    Ok(())
}
