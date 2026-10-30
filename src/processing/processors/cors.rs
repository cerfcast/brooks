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

//! The Mel Generic Metadata Processing Implementation

use crate::{
    cdni::cors::spec::TypedCrossoriginPolicy,
    processing::{
        self,
        imdi::ImdiResultValue,
        interpret::{
            self,
            Error::{self, AssertionFailure, NotImplemented},
            InterpretationResult, MetadataInformationResultElements,
        },
        processors::SimpleProcessorsInterpreterContext,
    },
    verifying::cors::verify::CrossoriginPolicyVerificationKey,
};

impl
    interpret::Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (ImdiResultValue, MetadataInformationResultElements),
        Error,
    > for TypedCrossoriginPolicy<CrossoriginPolicyVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (ImdiResultValue, MetadataInformationResultElements),
        Error,
    > {
        if let Some(preflight_only) = self.value.preflight_only
            && preflight_only
        {
            return Err(interpret::Error::RuntimeError(
                NotImplemented("Missing response when processing MI.CrossoriginPolicy".into())
                    .into(),
            )
            .into());
        }

        let mut res = match input.res {
            Some(res) => res,
            None => {
                return Err(interpret::Error::RuntimeError(
                    AssertionFailure(
                        "Missing response when processing MI.CrossoriginPolicy".into(),
                    )
                    .into(),
                )
                .into());
            }
        };
        let req = input.req;

        let origin_value = match req
            .headers()
            .iter()
            .find(|(name, _)| *name == http::header::ORIGIN.to_string())
        {
            Some(origin_value) => origin_value.1.clone(),
            None => todo!(),
        };

        let origin_value = origin_value.to_str().map_err(|e| {
            processing::interpret::Error::InvalidInput(
                format!("ORIGIN header value could not be converted to string: {e}").into(),
            )
        })?;

        // See whether the ORIGIN value in the request matches any of the
        // patterns.
        let apply = self
            .value
            .allow_origin
            .allow_origin
            .iter()
            .any(|v| v.aug.as_ref().unwrap().is_match(origin_value));

        // If any of the patterns matched the ORIGIN value,
        // set the ORIGIN header value in the response.
        if apply {
            if self.value.allow_origin.wildcard_return {
                res.set_header_value(http::header::ACCESS_CONTROL_ALLOW_ORIGIN.as_str(), "*")
                    .map_err(|e| interpret::Error::RuntimeError(e.into()))?;
            } else {
                res.set_header_value(
                    http::header::ACCESS_CONTROL_ALLOW_ORIGIN.as_str(),
                    origin_value,
                )
                .map_err(|e| interpret::Error::RuntimeError(e.into()))?;
            }
        }

        let updated_context = SimpleProcessorsInterpreterContext {
            scopes: input.scopes,
            mode: input.mode,
            runtime: input.runtime,
            req,
            res: Some(res),
        };

        Ok((
            updated_context,
            (None, MetadataInformationResultElements::default()),
        ))
    }

    fn at(&self, stage: interpret::Stages) -> bool {
        stage == interpret::Stages::Post
    }
}

#[cfg(test)]
mod test {}
