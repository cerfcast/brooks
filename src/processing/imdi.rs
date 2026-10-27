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

//! Interpret CDNI Interconnection Metadata

use std::fmt::Display;

use http::StatusCode;

use crate::environment::scope::Scopes;
use crate::logging::{LogLevel, LogMsg, LogMsgs};
use crate::processing::interpret::{Error, Interpreter, Stages};
use crate::verifying::verify::CdniVerificationKey;

use crate::processing::interpret::MetadataInformationResultElements;
use crate::processing::processors::SimpleProcessorsInterpreterContext;
use crate::processing::processors::ps::{PsInterpretMode, PsInterpretValue};
use crate::prr;
use crate::{cdni::imd::spec::HostMetadata, mel::interpreter::values::TypedValue};

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
///
/// TODO: Make clear in the documentation that this function is a _major_ entry point for library clients.
pub fn interpret_interconnection_metadata(
    hmd: &HostMetadata<CdniVerificationKey>,
    mel_scopes: Scopes<TypedValue>,
    request: Box<dyn prr::Prr<Vec<u8>>>,
    runtime: &tokio::runtime::Handle,
    mut log: LogMsgs,
) -> HmdInterpretResult {
    let mut request_context = SimpleProcessorsInterpreterContext {
        scopes: mel_scopes,
        mode: PsInterpretMode::Request,
        runtime,
        rr: request,
    };

    // Pre -- processors that want to do something _before_ processing, per se, starts.
    for hmd in &hmd.metadata {
        if hmd.aug.at(Stages::Pre) {
            (request_context, _) = match hmd.aug.interpret(request_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };
        }
    }

    // ClientRequest
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        #[allow(clippy::collapsible_if)]
        if stage.aug.at(Stages::ClientRequest) {
            // TODO: Handle a stage that generates MI.
            (request_context, _) = match stage.aug.interpret(request_context) {
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

    // OriginRequest
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if stage.aug.at(Stages::OriginRequest) {
            // TODO: Handle a stage that generates MI.
            (request_context, _) = match stage.aug.interpret(request_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };
        }
    }

    log = debug!(log, "Stop: processing origin request stages.");

    log = debug!(log, "Start: processing source stages.");

    let mut source_context = SimpleProcessorsInterpreterContext {
        scopes: request_context.scopes,
        mode: PsInterpretMode::Request,
        rr: request_context.rr,
        runtime: request_context.runtime,
    };
    let mut source_processor = false;

    // ClientRequest
    for hmd in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if hmd.aug.at(Stages::Source) {
            source_processor = true;
            (source_context, _) = match hmd.aug.interpret(source_context) {
                Ok((c, r)) => (c, r),
                Err(e) => {
                    return Err((MdInterpretError::MetadataProcessingError(e).into(), log));
                }
            };
            // There is only one Source processor allowed.
            break;
        }
    }

    log = debug!(
        log,
        &format!(
            "There was{}a source processor available.",
            if source_processor { " " } else { " not " }
        )
    );

    log = debug!(
        log,
        &format!(
            "There was{}data retrieved by the source processor.",
            if source_context.rr.tpe() == prr::PrrType::Response {
                " "
            } else {
                " not "
            }
        )
    );

    let mut response_context = SimpleProcessorsInterpreterContext {
        scopes: source_context.scopes,
        mode: PsInterpretMode::Response,
        rr: source_context.rr,
        runtime: source_context.runtime,
    };

    log = debug!(log, "Stop: processing source stages.");

    let mut response_result: Option<(PsInterpretValue, MetadataInformationResultElements)> = None;

    log = debug!(log, "Start: processing origin response stages.");
    // For any of the host metadata entries that are origin requests or origin responses,
    // do them now. Remember: The Request metadata objects can contain response transformations, too.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if stage.aug.at(Stages::OriginResponse) {
            let (updated_response_context, updated_result) =
                match stage.aug.interpret(response_context) {
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
        if stage.aug.at(Stages::ClientResponse) {
            let (updated_response_context, updated_result) = match stage
                .aug
                .interpret(response_context)
            {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((MdInterpretError::MetadataProcessingError(e).into(), log)),
            };

            if let PsInterpretValue::SyntheticResponse(sr) = updated_result.0 {
                log = debug!(
                    log,
                    "Got a synthetic response from an client response stage."
                );
                return Ok((sr.status(), sr, log));
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
