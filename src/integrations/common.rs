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

use chrono::Utc;
use http::Request;
use tokio::runtime::Runtime;

use crate::{
    cdni::{
        gmdp::ProcessedRequestResponse,
        md::{
            interpret::{
                HmdInterpretResultError, HmdInterpretValue, HmdTransformError, MdInterpretError,
                interpret_metadata,
            },
            spec::TypedHostMetadata,
            verify::verify_metadata,
        },
    },
    environment::scope::{Scope, Scopes},
    integrations::hmds::{HmdsConfiguration, query_hmds},
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{
        interpreter::interpret::TypedValue,
        scope::{builtin_function_types, minimal_core_variable_types},
        types::Type,
    },
    tools::prr,
};

pub(crate) fn safe_brooks_integration_handle(
    request: &Request<Vec<u8>>,
    mel: Scopes<TypedValue>,
    hmds_key: &str,
    hmds_config: &mut HmdsConfiguration,
    runtime: &Runtime,
    mut log: LogMsgs,
) -> Result<HmdInterpretValue, HmdInterpretResultError> {
    // First, try to find the query in the cache.
    log = debug!(
        log,
        &format!("Looking for {hmds_key} in the host metadata cache.")
    );

    let found = match hmds_config.hmds_cache.get(hmds_key) {
        Some((timeout, found)) => {
            log = debug!(
                log,
                &format!(
                    "Found {hmds_key} in the host metadata cache -- it is valid until {timeout}."
                )
            );
            if Utc::now() > *timeout {
                log = debug!(
                    log,
                    &format!("{hmds_key} in the host metadata cache timed out.")
                );
                hmds_config.hmds_cache.remove(hmds_key);
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
            let res = match runtime.block_on(query_hmds(hmds_key, &hmds_config.hmds_server)) {
                Ok(o) => o,
                Err(e) => {
                    return Err((MdInterpretError::HmdsQueryError(e.to_string()).into(), log));
                }
            };
            let (expiry, query_result) = match res {
                Some((timeout, query_result)) => (timeout, query_result),
                None => {
                    return Err((
                        MdInterpretError::MissingConfiguration(hmds_key.to_string()).into(),
                        log,
                    ));
                }
            };

            let metadata = match serde_json::from_value::<TypedHostMetadata<()>>(query_result) {
                Ok(o) => o,
                Err(e) => {
                    return Err((MdInterpretError::ProxyError(e.to_string()).into(), log));
                }
            };

            let types_scope = Scopes::<Type> {
                scopes: vec![&minimal_core_variable_types() + &builtin_function_types()],
            };

            let found = match verify_metadata(&metadata.value, types_scope) {
                Ok(o) => o,
                Err(e) => {
                    return Err((MdInterpretError::ProxyError(e.to_string()).into(), log));
                }
            };

            log = debug!(
                log,
                &format!("Put {hmds_key} in the host metadata cache to expire at {expiry}.")
            );
            hmds_config
                .hmds_cache
                .insert(hmds_key.to_string(), (expiry, found.clone()));

            found
        }
    };

    let processed_http_req = match TryInto::<ProcessedRequestResponse>::try_into(request) {
        Ok(o) => o,
        Err(e) => {
            return Err((
                MdInterpretError::TransformError(HmdTransformError::BadUrl(e.to_string()).into())
                    .into(),
                log,
            ));
        }
    };

    let processed_http_req = Box::new(processed_http_req);
    let mel_scopes: Scopes<TypedValue> = mel.enter_scope(Into::<Scope<TypedValue>>::into(
        &*processed_http_req as &dyn prr::Prr<Vec<u8>>,
    ));

    interpret_metadata(&found, mel_scopes, processed_http_req, runtime, log)
}
