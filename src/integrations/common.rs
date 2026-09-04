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

use std::{fmt::Display, str::FromStr};

use chrono::Utc;
use http::{HeaderName, HeaderValue, Request, StatusCode, Uri, header::HOST};
use reqwest::Url;
use tokio::runtime::Runtime;

use crate::{
    cdni::{
        spec::{HostMetadata, TypedHostMetadata},
        verify::{HostMetadataVerificationKey, verify_host_metadata},
    },
    integrations::hmds::{HmdsConfiguration, query_hmds},
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{
        interpreter::interpret::TypedValue,
        scope::{Scope, Scopes, builtin_function_types, minimal_core_variable_types},
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

#[derive(Debug)]
pub(crate) struct ProcessedResponse<'a> {
    pub res: &'a mut reqwest::Response,
    pub requested_uri: Uri,
    pub new_status: Option<StatusCode>,
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
pub(crate) struct ProcessedRequest<'a> {
    pub req: &'a mut Request<String>,
    pub updated_uri: Option<Uri>,
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

#[derive(Debug, Clone)]
pub enum BrooksIntegrationTransformError {
    BadHeaderName(String),
    BadHeaderValue(String),
    BadMethodValue(String),
    BadBody(String),
    BadUri(String),
    BadUrl(String),
    CreationError(String),
    BadMemory,
}

impl Display for BrooksIntegrationTransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrooksIntegrationTransformError::BadHeaderName(bhn) => {
                write!(f, "Bad header name: {bhn}")
            }
            BrooksIntegrationTransformError::BadHeaderValue(bhv) => {
                write!(f, "Bad header value: {bhv}")
            }
            BrooksIntegrationTransformError::BadMethodValue(bmv) => {
                write!(f, "Bad method value: {bmv}")
            }
            BrooksIntegrationTransformError::BadBody(bb) => write!(f, "Bad body: {bb}"),
            BrooksIntegrationTransformError::BadUri(bu) => write!(f, "Bad URI: {bu}"),
            BrooksIntegrationTransformError::BadUrl(bu) => write!(f, "Bad URL: {bu}"),
            BrooksIntegrationTransformError::CreationError(ce) => write!(f, "Creation error: {ce}"),
            BrooksIntegrationTransformError::BadMemory => {
                write!(f, "Pool memory allocation failed")
            }
        }
    }
}
#[derive(Debug)]
pub enum BrooksIntegrationsProxyError {
    TransformError(Box<BrooksIntegrationTransformError>),
    PsInterpretError(Box<PsInterpretError>),
    UpstreamError(String),
    MissingConfiguration(String),
    ProxyError(String),
    HmdsQueryError(String),
    RuntimeError(String),
    BadMemory,
}

impl Display for BrooksIntegrationsProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrooksIntegrationsProxyError::HmdsQueryError(query_error) => {
                write!(
                    f,
                    "Brooks Proxy Error: Error querying the HMDS: {query_error}"
                )
            }
            BrooksIntegrationsProxyError::TransformError(nginx_transform_error) => {
                write!(
                    f,
                    "Brooks Proxy Error: Transformation error: {nginx_transform_error}"
                )
            }
            BrooksIntegrationsProxyError::PsInterpretError(ps_interpret_error) => {
                write!(f, "Brooks Proxy Error: {ps_interpret_error}")
            }
            BrooksIntegrationsProxyError::UpstreamError(ue) => {
                write!(f, "Brooks Proxy Error: Upstream error: {ue}")
            }
            BrooksIntegrationsProxyError::ProxyError(pe) => write!(f, "Brooks Proxy Error: {pe}"),
            BrooksIntegrationsProxyError::RuntimeError(re) => {
                write!(f, "Brooks Proxy Error: Runtime error: {re}")
            }
            BrooksIntegrationsProxyError::MissingConfiguration(query) => {
                write!(f, "Brooks Proxy Error: Missing configuration for {query}")
            }
            BrooksIntegrationsProxyError::BadMemory => {
                write!(f, "Brooks Proxy Error: Pool memory allocation failed")
            }
        }
    }
}

pub(crate) fn safe_brooks_integration_handle(
    request: &mut Request<String>,
    mel: &Option<Scope<TypedValue>>,
    hmds_config: &mut HmdsConfiguration,
    runtime: &Runtime,
    log: &mut LogMsgs,
) -> Result<(StatusCode, reqwest::Response), Box<BrooksIntegrationsProxyError>> {
    let host = request
        .headers()
        .get(HOST)
        .ok_or(BrooksIntegrationsProxyError::ProxyError(
            "Could not get host from request".to_string(),
        ))?
        .to_str()
        .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

    // First, try to find the query in the cache.
    *log = debug!(
        log,
        &format!("Looking for {host} in the host metadata cache.")
    );

    let found = match hmds_config.hmds_cache.get(host) {
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
                hmds_config.hmds_cache.remove(host);
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
                .block_on(query_hmds(host, &hmds_config.hmds_server))
                .map_err(|e| BrooksIntegrationsProxyError::HmdsQueryError(e.to_string()))?
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
                scopes: vec![&minimal_core_variable_types() + &builtin_function_types()],
            };

            let found = verify_host_metadata(&metadata.value, types_scope)
                .map_err(|e| BrooksIntegrationsProxyError::ProxyError(e.to_string()))?;

            *log = debug!(
                log,
                &format!("Put {host} in the host metadata cache to expire at {expiry}.")
            );
            hmds_config
                .hmds_cache
                .insert(host.to_string(), (expiry, found.clone()));

            found
        }
    };

    let mut processed_http_req = ProcessedRequest {
        req: request,
        updated_uri: None,
    };

    safe_brooks_integrations_proxy(&found, mel, &mut processed_http_req, runtime, log)
}

pub(crate) fn safe_brooks_integrations_proxy(
    hmd: &HostMetadata<HostMetadataVerificationKey>,
    mel: &Option<Scope<TypedValue>>,
    processed_http_req: &mut ProcessedRequest,
    runtime: &tokio::runtime::Runtime,
    log: &mut LogMsgs,
) -> Result<(StatusCode, reqwest::Response), Box<BrooksIntegrationsProxyError>> {
    // For any of the host metadata entries that are client requests,
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let Some(stge) = &stage.aug.stage
            && TypedStageTypes::ClientRequest == stge.into()
        {
            interpret_stage(stge, mel, processed_http_req, PsInterpretMode::Request)
                .map_err(|e| Box::new(BrooksIntegrationsProxyError::PsInterpretError(e)))?;

            // There are no synthetic responses at this stage.
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
            interpret_stage(stge, mel, processed_http_req, PsInterpretMode::Request)
                .map_err(BrooksIntegrationsProxyError::PsInterpretError)?;

            // There are no synthetic responses at this stage.
        }
    }

    // Now, send the request to the origin.
    let processed_uri = processed_http_req.uri().map_err(|e| {
        BrooksIntegrationsProxyError::TransformError(Box::new(
            BrooksIntegrationTransformError::BadUri(e.to_string()),
        ))
    })?;

    let get_uri = Uri::from_str(&processed_uri.to_string()).map_err(|e| {
        BrooksIntegrationsProxyError::TransformError(Box::new(
            BrooksIntegrationTransformError::BadUri(e.to_string()),
        ))
    })?;

    let get_url = Url::from_str(&get_uri.to_string()).map_err(|e| {
        BrooksIntegrationsProxyError::TransformError(Box::new(
            BrooksIntegrationTransformError::BadUrl(e.to_string()),
        ))
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
        .map_err(|e| BrooksIntegrationsProxyError::UpstreamError(e.to_string()))?;

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
            let result = interpret_stage(
                stge,
                mel,
                &mut processed_http_res,
                PsInterpretMode::Response,
            )
            .map_err(BrooksIntegrationsProxyError::PsInterpretError)?;

            if let PsInterpretValue::SyntheticResponse(sr) = result.0 {
                *log = debug!(
                    log,
                    "Got a synthetic response from an origin response stage."
                );
                return Ok((
                    processed_http_res.status(),
                    sr.clone().map(reqwest::Body::from).into(),
                ));
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
            let result = interpret_stage(
                stge,
                mel,
                &mut processed_http_res,
                PsInterpretMode::Response,
            )
            .map_err(BrooksIntegrationsProxyError::PsInterpretError)?;

            if let PsInterpretValue::SyntheticResponse(sr) = result.0 {
                *log = debug!(
                    log,
                    "Got a synthetic response from an client response stage."
                );
                return Ok((
                    processed_http_res.status(),
                    sr.clone().map(reqwest::Body::from).into(),
                ));
            }
        }
    }

    Ok((processed_http_res.status(), result))
}
