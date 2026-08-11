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
    collections::HashMap, fmt::Display, marker::PhantomData, path::PathBuf, ptr::null, str::FromStr,
};

use chrono::Utc;
use http::{HeaderName, HeaderValue, Method, Request, StatusCode, Uri, header::HOST};
use libc::intptr_t;
use reqwest::{Response, Url};
use tokio::runtime;

use crate::{
    cdni::{
        spec::{HostMetadata, TypedHostMetadata},
        verify::{HostMetadataVerificationKey, verify_host_metadata},
    },
    integrations::{
        hmds::query_hmds,
        nginx::{
            nginx_lib::{from_nginx_str, log_nginx_msgs, to_nginx_buf, to_nginx_str},
            ngx_buf_s, ngx_http_request_s, ngx_list_push, ngx_log_s, ngx_str_t, ngx_table_elt_s,
        },
    },
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{
        scope::{Scopes, minimal_core_variable_types},
        tvs::Type,
    },
    ps::{
        interpret::{
            ProcessableRequestResponse,
            ProcessableRequestResponseError::{BadValue, InvalidMode},
            ProcessableRequestResponseResult, PsInterpretError, PsInterpretMode, PsInterpretValue,
            interpret_stage,
        },
        spec::TypedStageTypes,
    },
};

#[repr(C)]
pub struct BrooksC {
    hmds_path: PathBuf,
    hmds_cache: HashMap<
        String,
        (
            chrono::DateTime<Utc>,
            HostMetadata<HostMetadataVerificationKey>,
        ),
    >,
    _marker: core::marker::PhantomData<*mut u8>,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_configure(
    cookie: *mut *const BrooksC,
    hmds_path: ngx_str_t,
    nlx: *mut ngx_log_s,
) -> bool {
    let log = LogMsgs::new_with_prefix("brooks analysis", crate::logging::LogLevel::Debug);

    *cookie = Box::into_raw(Box::new(BrooksC {
        hmds_path: PathBuf::from(from_nginx_str(hmds_path)),
        hmds_cache: HashMap::new(),
        _marker: PhantomData {},
    }));

    log_nginx_msgs(nlx, &log);

    true
}

#[derive(Debug, Clone)]
pub enum NginxTransformError {
    BadHeaderName(String),
    BadHeaderValue(String),
    BadMethodValue(String),
    BadBody(String),
    BadUri(String),
    BadUrl(String),
    CreationError(String),
    BadMemory,
}

impl Display for NginxTransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NginxTransformError::BadHeaderName(bhn) => write!(f, "Bad header name: {bhn}"),
            NginxTransformError::BadHeaderValue(bhv) => write!(f, "Bad header value: {bhv}"),
            NginxTransformError::BadMethodValue(bmv) => write!(f, "Bad method value: {bmv}"),
            NginxTransformError::BadBody(bb) => write!(f, "Bad body: {bb}"),
            NginxTransformError::BadUri(bu) => write!(f, "Bad URI: {bu}"),
            NginxTransformError::BadUrl(bu) => write!(f, "Bad URL: {bu}"),
            NginxTransformError::CreationError(ce) => write!(f, "Creation error: {ce}"),
            NginxTransformError::BadMemory => write!(f, "Pool memory allocation failed"),
        }
    }
}

#[derive(Debug)]
pub enum NginxProxyError {
    TransformError(Box<NginxTransformError>),
    PsInterpretError(Box<PsInterpretError>),
    UpstreamError(String),
    MissingConfiguration(String),
    ProxyError(String),
    RuntimeError(String),
    BadMemory,
}

impl Display for NginxProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NginxProxyError::TransformError(nginx_transform_error) => {
                write!(f, "Brooks Proxy Error: {nginx_transform_error}")
            }
            NginxProxyError::PsInterpretError(ps_interpret_error) => {
                write!(f, "Brooks Proxy Error: {ps_interpret_error}")
            }
            NginxProxyError::UpstreamError(ue) => write!(f, "Brooks Proxy Error: {ue}"),
            NginxProxyError::ProxyError(pe) => write!(f, "Brooks Proxy Error: {pe}"),
            NginxProxyError::RuntimeError(re) => write!(f, "Brooks Proxy Error: {re}"),
            NginxProxyError::MissingConfiguration(query) => {
                write!(f, "Brooks Proxy Error: Missing configuration for {query}")
            }
            NginxProxyError::BadMemory => {
                write!(f, "Brooks Proxy Error: Pool memory allocation failed")
            }
        }
    }
}

impl TryFrom<ngx_http_request_s> for Request<String> {
    type Error = Box<NginxTransformError>;

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
                    HeaderName::from_str(&k).map_err(|_| NginxTransformError::BadHeaderName(k))?,
                    HeaderValue::from_str(&val)
                        .map_err(|_| NginxTransformError::BadHeaderValue(val))?,
                );

                header_element = header_element.wrapping_add(1);
                i += 1;
            }

            let parsed_uri = Uri::from_str(&format!(
                "{}?{}",
                from_nginx_str(value.uri),
                from_nginx_str(value.args)
            ))
            .map_err(|e| NginxTransformError::BadUri(e.to_string()))?;

            request = request.uri(parsed_uri.clone());

            request = request.method(
                Method::from_str(&from_nginx_str(value.method_name))
                    .map_err(|e| NginxTransformError::BadMethodValue(e.to_string()))?,
            );
        }

        request
            .body("".to_string())
            .map_err(|e| Box::new(NginxTransformError::BadBody(e.to_string())))
    }
}

unsafe fn try_from_response(
    response: &Response,
    status: StatusCode,
    req: *mut ngx_http_request_s,
) -> Result<(), Box<NginxTransformError>> {
    for (hsh, header) in response.headers().iter().enumerate() {
        let header_name = header.0.to_string();
        let header_value = header
            .1
            .to_str()
            .map_err(|e| NginxTransformError::BadHeaderValue(e.to_string()))?;

        let he = ngx_list_push(&mut (*req).headers_out.headers) as *mut ngx_table_elt_s;

        (*he).hash = hsh;
        (*he).key = to_nginx_str(&header_name, (*req).pool);
        (*he).value = to_nginx_str(header_value, (*req).pool);
    }

    (*req).headers_out.status = status.as_u16() as usize;

    Ok(())
}

#[derive(Debug)]
struct ProcessedResponse<'a> {
    res: &'a mut reqwest::Response,
    requested_uri: Uri,
    new_status: Option<StatusCode>,
}

impl<'a> ProcessedResponse<'a> {
    pub fn status(&self) -> StatusCode {
        if let Some(new_status) = self.new_status {
            new_status
        } else {
            self.res.status()
        }
    }
}

impl<'a> ProcessableRequestResponse for ProcessedResponse<'a> {
    fn header_value(&self) -> Option<String> {
        todo!()
    }

    fn headers(&self) -> Vec<String> {
        self.res.headers().keys().map(|hv| hv.to_string()).collect()
    }

    fn set_header_value(
        &mut self,
        header: &str,
        value: &str,
    ) -> ProcessableRequestResponseResult<()> {
        self.res.headers_mut().insert(
            HeaderName::from_str(header).map_err(|_| BadValue)?,
            HeaderValue::from_str(value).map_err(|_| BadValue)?,
        );
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> ProcessableRequestResponseResult<()> {
        self.res
            .headers_mut()
            .remove(HeaderName::from_str(header).map_err(|_| BadValue)?);
        Ok(())
    }

    fn add_header(&mut self, header: &str, value: &str) -> ProcessableRequestResponseResult<()> {
        self.set_header_value(header, value)
    }

    fn uri(&self) -> ProcessableRequestResponseResult<Uri> {
        Ok(self.requested_uri.clone())
    }

    fn set_uri(&mut self, _uri: &Uri) -> ProcessableRequestResponseResult<()> {
        Err(InvalidMode)
    }

    fn set_response(&mut self, response: &u16) -> ProcessableRequestResponseResult<()> {
        self.new_status = Some(StatusCode::from_u16(*response).map_err(|_| BadValue)?);
        Ok(())
    }

    fn clear_headers(&mut self) -> ProcessableRequestResponseResult<()> {
        self.res.headers_mut().clear();
        Ok(())
    }
}

#[derive(Debug)]
struct ProcessedRequest<'a> {
    req: &'a mut Request<String>,
    updated_uri: Option<Uri>,
}

impl<'a> ProcessableRequestResponse for ProcessedRequest<'a> {
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

    fn uri(&self) -> ProcessableRequestResponseResult<Uri> {
        if let Some(uri) = &self.updated_uri {
            Ok(uri.clone())
        } else {
            Ok(self.req.uri().clone())
        }
    }

    fn set_uri(&mut self, uri: &Uri) -> ProcessableRequestResponseResult<()> {
        self.updated_uri = Some(uri.clone());

        // When the caller updates the URI, make sure to update the request headers.
        self.req.headers_mut().insert(
            http::header::HOST,
            HeaderValue::from_str(uri.authority().ok_or(BadValue)?.as_ref())
                .map_err(|_| BadValue)?,
        );
        Ok(())
    }

    fn set_response(&mut self, _response: &u16) -> ProcessableRequestResponseResult<()> {
        Err(InvalidMode)
    }

    fn clear_headers(&mut self) -> ProcessableRequestResponseResult<()> {
        self.req.headers_mut().clear();
        Ok(())
    }
}

enum NginxReturnCodes {
    Ok = 0,
    Error = -1,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_proxy(
    cookie: *mut BrooksC,
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
    cookie: *mut BrooksC,
    req: *mut ngx_http_request_s,
    body: *mut *mut ngx_buf_s,
    log: &mut LogMsgs,
) -> Result<(), Box<NginxProxyError>> {
    let mut http_req = TryInto::<Request<String>>::try_into(*req)
        .map_err(|e| Box::new(NginxProxyError::TransformError(e)))?;

    let host = http_req
        .headers()
        .get(HOST)
        .ok_or(NginxProxyError::ProxyError(
            "Could not get host from request".to_string(),
        ))?
        .to_str()
        .map_err(|e| NginxProxyError::ProxyError(e.to_string()))?;

    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| NginxProxyError::RuntimeError(e.to_string()))?;

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
                .map_err(|e| NginxProxyError::ProxyError(e.to_string()))?
            {
                Some((timeout, query_result)) => (timeout, query_result),
                None => return Err(NginxProxyError::MissingConfiguration(host.to_string()).into()),
            };

            let metadata = serde_json::from_value::<TypedHostMetadata<()>>(query_result)
                .map_err(|e| NginxProxyError::ProxyError(e.to_string()))?;

            let types_scope = Scopes::<Type> {
                scopes: vec![minimal_core_variable_types()],
            };
            let found = verify_host_metadata(&metadata.value, types_scope)
                .map_err(|e| NginxProxyError::ProxyError(e.to_string()))?;

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

    let (status, response) = safe_ngx_brooks_proxy(&found, &mut processed_http_req, &runtime, log)?;

    try_from_response(&response, status, req).map_err(NginxProxyError::TransformError)?;

    let result_body = runtime
        .block_on(response.bytes())
        .map_err(|e| NginxProxyError::ProxyError(e.to_string()))?;

    *body = to_nginx_buf(&result_body, (*req).pool).map_err(NginxProxyError::TransformError)?;

    (*req).headers_out.content_length_n = result_body.len() as i64;

    Ok(())
}

fn safe_ngx_brooks_proxy(
    hmd: &HostMetadata<HostMetadataVerificationKey>,
    processed_http_req: &mut ProcessedRequest,
    runtime: &tokio::runtime::Runtime,
    log: &mut LogMsgs,
) -> Result<(StatusCode, Response), Box<NginxProxyError>> {
    // For any of the host metadata entries that are client requests,
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let Some(stge) = &stage.aug.stage
            && TypedStageTypes::ClientRequest == stge.into()
        {
            let result = interpret_stage(stge, processed_http_req, PsInterpretMode::Request)
                .map_err(|e| Box::new(NginxProxyError::PsInterpretError(e)))?;

            if let PsInterpretValue::SyntheticResponse(_sr) = result {
                *log = debug!(log, "Got a synthetic response");
                todo!("Handle Synthetic Responses")
            }
        }
    }

    // If there were a cache, we would access it here.
    if false {
        todo!("Implement caching.")
    }

    // For any of the host metadata entries that are origin requests,
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let Some(stge) = &stage.aug.stage
            && TypedStageTypes::OriginRequest == stge.into()
        {
            let result = interpret_stage(stge, processed_http_req, PsInterpretMode::Request)
                .map_err(NginxProxyError::PsInterpretError)?;

            if let PsInterpretValue::SyntheticResponse(_sr) = result {
                *log = debug!(log, "Got a synthetic response");
                todo!("Handle Synthetic Responses")
            }
        }
    }

    // Now, send the request to the origin.
    let processed_uri = processed_http_req.uri().map_err(|e| {
        NginxProxyError::TransformError(Box::new(NginxTransformError::BadUri(e.to_string())))
    })?;

    let get_uri = Uri::from_str(&processed_uri.to_string()).map_err(|e| {
        NginxProxyError::TransformError(Box::new(NginxTransformError::BadUri(e.to_string())))
    })?;

    let get_url = Url::from_str(&get_uri.to_string()).map_err(|e| {
        NginxProxyError::TransformError(Box::new(NginxTransformError::BadUrl(e.to_string())))
    })?;

    let mut proxy_request = reqwest::Client::new().get(get_url.clone());

    for (name, value) in processed_http_req.req.headers() {
        proxy_request = proxy_request.header(name, value);
    }

    let mut proxy_request = reqwest::Client::new().get(get_url.clone());
    for (name, value) in processed_http_req.req.headers() {
        proxy_request = proxy_request.header(name, value);
    }
    let mut result = runtime
        .block_on(proxy_request.send())
        .map_err(|e| NginxProxyError::UpstreamError(e.to_string()))?;

    let mut processed_http_res = ProcessedResponse {
        requested_uri: get_uri,
        res: &mut result,
        new_status: None,
    };

    // For any of the host metadata entries that are origin requests or origin responses,
    // do them now. Remember: The *Request metadata objects can contain response transformations, too.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let Some(stge) = &stage.aug.stage
            && (TypedStageTypes::OriginRequest == stge.into()
                || TypedStageTypes::OriginResponse == stge.into())
        {
            let result = interpret_stage(stge, &mut processed_http_res, PsInterpretMode::Response)
                .map_err(NginxProxyError::PsInterpretError)?;

            if let PsInterpretValue::SyntheticResponse(_sr) = result {
                *log = debug!(log, "Got a synthetic response");
                todo!("Handle Synthetic Responses")
            }
        }
    }

    // If there were a cache, we would updated it here.
    if false {
        todo!("Implement caching.")
    }

    // For any of the host metadata entries that are client requests or client responses,
    // do them now. Remember: The *Client metadata objects can contain response transformations, too.
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let Some(stge) = &stage.aug.stage
            && (TypedStageTypes::ClientResponse == stge.into()
                || TypedStageTypes::ClientRequest == stge.into())
        {
            let result = interpret_stage(stge, &mut processed_http_res, PsInterpretMode::Response)
                .map_err(NginxProxyError::PsInterpretError)?;

            if let PsInterpretValue::SyntheticResponse(_sr) = result {
                *log = debug!(log, "Got a synthetic response");
                todo!("Handle Synthetic Responses")
            }
        }
    }

    Ok((processed_http_res.status(), result))
}
