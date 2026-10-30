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

use crate::environment::scope::Scopes;
use crate::logging::{LogLevel, LogMsg, LogMsgs};
use crate::processing::error::{ImdInterpretError, ImdInterpretResult};
use crate::processing::interpret::{Interpreter, Stages};
use crate::verifying::verify::CdniVerificationKey;

use crate::processing::processors::SimpleProcessorsInterpreterContext;
use crate::processing::processors::ps::{PsInterpretMode, PsInterpretValue};
use crate::prr;
use crate::{cdni::imd::spec::HostMetadata, mel::interpreter::values::TypedValue};

/// Interpret given Host Metadata for the given request.
///
/// TODO: Make clear in the documentation that this function is a _major_ entry point for library clients.
pub fn interpret_interconnection_metadata(
    hmd: &HostMetadata<CdniVerificationKey>,
    mel_scopes: Scopes<TypedValue>,
    request: Box<dyn prr::Prr<Vec<u8>>>,
    runtime: &tokio::runtime::Handle,
    mut log: LogMsgs,
) -> ImdInterpretResult {
    let mut request_context = SimpleProcessorsInterpreterContext {
        scopes: mel_scopes,
        mode: PsInterpretMode::Request,
        runtime,
        req: request,
        res: None,
    };

    // Pre -- processors that want to do something _before_ processing, per se, starts.
    for hmd in &hmd.metadata {
        if hmd.aug.at(Stages::Pre) {
            (request_context, _) = match hmd.aug.interpret(request_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((ImdInterpretError::MetadataProcessingError(e).into(), log)),
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
                Err(e) => return Err((ImdInterpretError::MetadataProcessingError(e).into(), log)),
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
                Err(e) => return Err((ImdInterpretError::MetadataProcessingError(e).into(), log)),
            };
        }
    }

    log = debug!(log, "Stop: processing origin request stages.");

    log = debug!(log, "Start: processing source stages.");

    let mut source_context = SimpleProcessorsInterpreterContext {
        scopes: request_context.scopes,
        mode: PsInterpretMode::Request,
        req: request_context.req,
        res: None,
        runtime: request_context.runtime,
    };

    let mut source_processor = false;

    for hmd in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if hmd.aug.at(Stages::Source) {
            source_processor = true;
            (source_context, _) = match hmd.aug.interpret(source_context) {
                Ok((c, r)) => (c, r),
                Err(e) => {
                    return Err((ImdInterpretError::MetadataProcessingError(e).into(), log));
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
            if let Some(res) = &source_context.res
                && res.tpe() == prr::PrrType::Response
            {
                " "
            } else {
                " not "
            }
        )
    );

    // TODO: Handle no source processor!

    let mut response_context = SimpleProcessorsInterpreterContext {
        scopes: source_context.scopes,
        mode: PsInterpretMode::Response,
        req: source_context.req,
        res: source_context.res,
        runtime: source_context.runtime,
    };

    log = debug!(log, "Stop: processing source stages.");

    log = debug!(log, "Start: processing origin response stages.");
    // For any of the host metadata entries that are origin requests or origin responses,
    // do them now. Remember: The Request metadata objects can contain response transformations, too.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if stage.aug.at(Stages::OriginResponse) {
            let (updated_response_context, result) = match stage.aug.interpret(response_context) {
                Ok((c, r)) => (c, r),
                Err(e) => {
                    return Err((ImdInterpretError::MetadataProcessingError(e).into(), log));
                }
            };

            // Handle a result, if any.
            if let PsInterpretValue::SyntheticResponse(sr) = result.0 {
                log = debug!(
                    log,
                    "Got a synthetic response from an origin response stage."
                );
                return Ok((sr.status(), sr, log));
            }

            response_context = updated_response_context;

            // TODO: Handle other values from the PS Interpreter that indicate
            // that processing should stop.
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
            let (updated_response_context, result) = match stage.aug.interpret(response_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((ImdInterpretError::MetadataProcessingError(e).into(), log)),
            };

            // Handle a result, if any.
            if let PsInterpretValue::SyntheticResponse(sr) = result.0 {
                log = debug!(
                    log,
                    "Got a synthetic response from an client response stage."
                );
                return Ok((sr.status(), sr, log));
            }

            // TODO: Handle other values from the PS Interpreter that indicate
            // that processing should stop.

            response_context = updated_response_context;
        }
    }

    log = debug!(log, "Stop: processing client response stages.");

    log = debug!(log, "Start: processing post stages.");

    // For any of the host metadata entries that are client requests or client responses,
    // do them now. Remember: The *Client metadata objects can contain response transformations, too.
    // do them now.
    for stage in &hmd.metadata {
        // TODO: Determine if/when/how processing will stop when there is a terminating metadata object.
        if stage.aug.at(Stages::Post) {
            let (updated_response_context, _) = match stage.aug.interpret(response_context) {
                Ok((c, r)) => (c, r),
                Err(e) => return Err((ImdInterpretError::MetadataProcessingError(e).into(), log)),
            };

            // TODO: Handle other values from the PS Interpreter that indicate
            // that processing should stop.

            response_context = updated_response_context;
        }
    }
    log = debug!(log, "Stop: processing post stages.");

    log = debug!(log, "Trying to convert a response into a response!");

    let processed_http_response = match response_context.res {
        Some(res) => res,
        None => todo!(),
    };
    let interpreted_http_response: http::Response<Vec<u8>> =
        match (&*processed_http_response as &dyn prr::Prr<Vec<u8>>).try_into() {
            Ok(o) => o,
            Err(e) => {
                return Err((ImdInterpretError::TransformError(e.into()).into(), log));
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
