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

//! Interpret CDNI Metadata

use std::fmt::Display;

use http::StatusCode;

use crate::cdni::gmd::spec::TypedSource;
use crate::cdni::gmdp::{Error, Interpreter};
use crate::cdni::md::verify::CdniVerificationKey;
use crate::cdni::mi::MetadataInformationResultElements;
use crate::cdni::processors::SimpleProcessorsInterpreterContext;
use crate::environment::scope::Scopes;
use crate::logging::{LogLevel, LogMsg, LogMsgs};

use crate::tools::prr;
use crate::{
    cdni::{
        md::spec::HostMetadata,
        ps::{
            interpret::{PsInterpretMode, PsInterpretValue},
            spec::TypedStageTypes,
        },
    },
    mel::interpreter::interpret::TypedValue,
};

pub type HmdInterpretValue = (StatusCode, http::Response<Vec<u8>>, LogMsgs);
pub type HmdInterpretResultError = (Box<MdInterpretError>, LogMsgs);
pub type HmdInterpretResult = Result<HmdInterpretValue, (Box<MdInterpretError>, LogMsgs)>;

#[derive(Debug, Clone)]
pub enum HmdTransformError {
    BadHeaderName(String),
    BadHeaderValue(String),
    BadMethodValue(String),
    BadBody(String),
    BadUri(String),
    BadUrl(String),
    CreationError(String),
    BadMemory,
}

impl Display for HmdTransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HmdTransformError::BadHeaderName(bhn) => {
                write!(f, "Bad header name: {bhn}")
            }
            HmdTransformError::BadHeaderValue(bhv) => {
                write!(f, "Bad header value: {bhv}")
            }
            HmdTransformError::BadMethodValue(bmv) => {
                write!(f, "Bad method value: {bmv}")
            }
            HmdTransformError::BadBody(bb) => write!(f, "Bad body: {bb}"),
            HmdTransformError::BadUri(bu) => write!(f, "Bad URI: {bu}"),
            HmdTransformError::BadUrl(bu) => write!(f, "Bad URL: {bu}"),
            HmdTransformError::CreationError(ce) => write!(f, "Creation error: {ce}"),
            HmdTransformError::BadMemory => {
                write!(f, "Pool memory allocation failed")
            }
        }
    }
}

#[derive(Debug)]
pub enum MdInterpretError {
    TransformError(Box<HmdTransformError>),
    ProcessableRequestResponseError(Box<prr::Error>),
    UpstreamError(String),
    MissingConfiguration(String),
    ProxyError(String),
    HmdsQueryError(String),
    RuntimeError(String),
    MetadataProcessingError(Box<Error>),
    BadMemory,
}

impl Display for MdInterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MdInterpretError::HmdsQueryError(query_error) => {
                write!(
                    f,
                    "Brooks Proxy Error: Error querying the HMDS: {query_error}"
                )
            }
            MdInterpretError::TransformError(nginx_transform_error) => {
                write!(
                    f,
                    "Brooks Proxy Error: Transformation error: {nginx_transform_error}"
                )
            }
            MdInterpretError::MetadataProcessingError(ps_interpret_error) => {
                write!(f, "Brooks Proxy Error: {}", *ps_interpret_error)
            }
            MdInterpretError::UpstreamError(ue) => {
                write!(f, "Brooks Proxy Error: Upstream error: {ue}")
            }
            MdInterpretError::ProxyError(pe) => write!(f, "Brooks Proxy Error: {pe}"),
            MdInterpretError::RuntimeError(re) => {
                write!(f, "Brooks Proxy Error: Runtime error: {re}")
            }
            MdInterpretError::MissingConfiguration(query) => {
                write!(f, "Brooks Proxy Error: Missing configuration for {query}")
            }
            MdInterpretError::BadMemory => {
                write!(f, "Brooks Proxy Error: Pool memory allocation failed")
            }
            MdInterpretError::ProcessableRequestResponseError(
                processable_request_response_error,
            ) => {
                write!(
                    f,
                    "Brooks Proxy Error: Error converting from the processed request/response to the request/response: {processable_request_response_error}"
                )
            }
        }
    }
}

/// Interpret given Host Metadata for the given request.
pub fn interpret_metadata(
    hmd: &HostMetadata<CdniVerificationKey>,
    mel_scopes: Scopes<TypedValue>,
    request: Box<dyn prr::Prr<Vec<u8>>>,
    runtime: &tokio::runtime::Runtime,
    mut log: LogMsgs,
) -> HmdInterpretResult {
    let mut request_context = SimpleProcessorsInterpreterContext {
        scopes: mel_scopes,
        mode: PsInterpretMode::Request,
        runtime,
        rr: request,
    };

    // For any of the host metadata entries that are client requests,
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        #[allow(clippy::collapsible_if)]
        if let CdniVerificationKey::Stage(stge) = &stage.aug
            && TypedStageTypes::ClientRequest == (&**stge).into()
        {
            // TODO: Handle a stage that generates MI.
            (request_context, _) = match stge.interpret(request_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };

            // There are no synthetic responses at this stage.
        }
    }

    // If there were a cache, we would access it here.
    if false {
        todo!("Implement caching.")
    }

    log = debug!(log, "Start: processing origin request stages.");

    // For any of the host metadata entries that are origin requests,
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        #[allow(clippy::collapsible_if)]
        if let CdniVerificationKey::Stage(stge) = &stage.aug
            && TypedStageTypes::OriginRequest == (&**stge).into()
        {
            // TODO: Handle a stage that generates MI.
            (request_context, _) = match stge.interpret(request_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };
        }
    }

    log = debug!(log, "Stop: processing origin request stages.");

    log = debug!(log, "Start: processing source stages.");

    let source_context = SimpleProcessorsInterpreterContext {
        scopes: request_context.scopes,
        mode: PsInterpretMode::Request,
        rr: request_context.rr,
        runtime: request_context.runtime,
    };

    let source_stage_processing_result = hmd
        .metadata
        .iter()
        .map_while(|stage| {
            if let CdniVerificationKey::Source(s) = &stage.aug {
                Some(s)
            } else {
                None
            }
        })
        .fold(
            (Some(source_context), None),
            |(source_context, result), next| {
                if result.is_none()
                    && let Some(source_context) = source_context
                {
                    match next.interpret(source_context) {
                        Ok((updated_context, result)) => (Some(updated_context), Some(Ok(result))),
                        Err(e) => (
                            None,
                            Some(Err(MdInterpretError::MetadataProcessingError(e))),
                        ),
                    }
                } else {
                    (source_context, result)
                }
            },
        );

    log = debug!(log, "Done: processing source stages.");

    // TODO: Handle Source stages that generate MI.
    let (source_result_context, (_source_result_psiv, _source_result_mdire)) =
        match source_stage_processing_result {
            (Some(source_result_context), Some(Ok(result))) => (source_result_context, result),
            (Some(_), Some(Err(e))) => {
                return Err((MdInterpretError::RuntimeError(e.to_string()).into(), log));
            }
            (_, _) => {
                return Err((
                    MdInterpretError::MetadataProcessingError(
                        Error::NoProcessor(TypedSource::<()>::typed_cdni_metadata_name()).into(),
                    )
                    .into(),
                    log,
                ));
            }
        };

    log = debug!(log, "A result exists from the source!");

    // Check that the resulting processable request/response has the right type.

    if source_result_context.rr.tpe() != prr::PrrType::Response {
        return Err((
            MdInterpretError::MetadataProcessingError(
                Error::InvalidInput(prr::Error::InvalidMode.into()).into(),
            )
            .into(),
            log,
        ));
    };

    log = debug!(log, "The result from the source is a result!");

    let mut response_context = SimpleProcessorsInterpreterContext {
        scopes: source_result_context.scopes,
        mode: PsInterpretMode::Response,
        rr: source_result_context.rr,
        runtime: source_result_context.runtime,
    };
    let mut response_result: Option<(PsInterpretValue, MetadataInformationResultElements)> = None;

    log = debug!(log, "Start: processing origin response stages.");
    // For any of the host metadata entries that are origin requests or origin responses,
    // do them now. Remember: The *Request metadata objects can contain response transformations, too.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let CdniVerificationKey::Stage(stge) = &stage.aug
            && (TypedStageTypes::OriginRequest == (&**stge).into()
                || TypedStageTypes::OriginResponse == (&**stge).into())
        {
            let (updated_response_context, updated_result) = match stge.interpret(response_context)
            {
                Ok((c, r)) => (c, r),
                Err(e) => {
                    return Err((MdInterpretError::MetadataProcessingError(e).into(), log));
                }
            };

            if let PsInterpretValue::SyntheticResponse(sr) = updated_result.0 {
                log = debug!(
                    log,
                    "Got a synthetic response from an origin response stage."
                );
                return Ok((
                    updated_response_context.rr.get_status().expect("TODO"),
                    sr,
                    log,
                ));
            }

            // TODO: Handle other values from the PS Interpreter that indicate
            // that processing should stop.

            response_context = updated_response_context;
            response_result = Some(updated_result);
        }
    }
    log = debug!(log, "Stop: processing origin response stages.");

    // If there were a cache, we would updated it here.
    if false {
        todo!("Implement caching.")
    }

    log = debug!(log, "Start: processing client response stages.");
    // For any of the host metadata entries that are client requests or client responses,
    // do them now. Remember: The *Client metadata objects can contain response transformations, too.
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if let CdniVerificationKey::Stage(stge) = &stage.aug
            && (TypedStageTypes::ClientResponse == (&**stge).into()
                || TypedStageTypes::ClientRequest == (&**stge).into())
        {
            let (updated_response_context, updated_result) = match stge.interpret(response_context)
            {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };

            if let PsInterpretValue::SyntheticResponse(sr) = updated_result.0 {
                log = debug!(
                    log,
                    "Got a synthetic response from an client response stage."
                );
                return Ok((
                    updated_response_context.rr.get_status().expect("TODO"),
                    sr,
                    log,
                ));
            }

            // TODO: Handle other values from the PS Interpreter that indicate
            // that processing should stop.

            response_context = updated_response_context;
            response_result = Some(updated_result);
        }
    }

    // TODO: Handle response stages that generate MI.
    let (_interpreted_psiv, _interpreted_mdire) = match response_result {
        Some(o) => o,
        None => {
            return Err((
                MdInterpretError::MetadataProcessingError(
                    Error::NoProcessor(
                        hmd.metadata
                            .iter()
                            .map(|md| md.tpe.clone())
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                    .into(),
                )
                .into(),
                log,
            ));
        }
    };

    log = debug!(log, "Stop: processing client response stages.");

    log = debug!(log, "Trying to convert a response into a response!");

    let interpreted_http_response: http::Response<Vec<u8>> =
        match (&*response_context.rr as &dyn prr::Prr<Vec<u8>>).try_into() {
            Ok(o) => o,
            Err(e) => {
                return Err((
                    MdInterpretError::ProcessableRequestResponseError(e.into()).into(),
                    log,
                ));
            }
        };

    log = debug!(
        log,
        &format!("Response code: {}", interpreted_http_response.status())
    );

    Ok((
        interpreted_http_response.status(),
        interpreted_http_response,
        log,
    ))
}
