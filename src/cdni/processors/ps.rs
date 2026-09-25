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

//! The Processing Stages Generic Metadata Processing Implementation

use crate::cdni::{
    gmd,
    gmdp::{InterpretationResult, Interpreter, Verifier},
    md::verify::CdniVerificationKey,
    mi::MetadataInformationResultElements,
    processors::{SimpleProcessorsAnalysisContext, SimpleProcessorsInterpreterContext},
    ps::{
        interpret::{
            PsInterpretAssertionFailures, PsInterpretContext, PsInterpretError, PsInterpretValue,
            PsInterpreter,
        },
        spec::{
            TypedClientRequestStage, TypedClientResponseStage, TypedGenericStage,
            TypedOriginRequestStage, TypedOriginResponseStage, TypedStage,
        },
        verify::{PsVerificationKey, verify_ps_request_stage},
        visit::PsVisitor,
    },
};
use crate::cdni::{
    gmdp::{AnalysisResult, Error, Stages},
    md::verify::HostMetadataVerificationError,
};

use std::fmt::Debug;

#[derive(Debug)]
pub struct PsMetadataAnalyzer {}

impl
    Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > for TypedStage<PsVerificationKey>
{
    fn interpret<'a>(
        &self,
        mut input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > {
        let reqres = &mut *input.rr;

        let mut visitor = PsInterpreter { rr: reqres };

        visitor.install_generic_visitors();

        let context = PsInterpretContext {
            mode: input.mode.clone(),
            mel_scopes: input.scopes.clone(),
            ..Default::default()
        };

        let result = match self {
            TypedStage::ClientRequest(typed_client_request_stage) => {
                visitor.visit_client_request_stage(typed_client_request_stage, context)
            }
            TypedStage::ClientResponse(typed_client_response_stage) => {
                visitor.visit_client_response_stage(typed_client_response_stage, context)
            }
            TypedStage::OriginRequest(typed_origin_request_stage) => {
                visitor.visit_origin_request_stage(typed_origin_request_stage, context)
            }
            TypedStage::OriginResponse(typed_origin_response_stage) => {
                visitor.visit_origin_response_stage(typed_origin_response_stage, context)
            }
        }
        .map_err(|e| Error::RuntimeError(e.into()))?;

        let md = result.metadata;
        let result = result.result.ok_or(Error::RuntimeError(
            PsInterpretError::AssertionFailure(PsInterpretAssertionFailures::MissingResult).into(),
        ))?;
        Ok((input, (result, md)))
    }

    fn stage(&self, stage: Stages) -> bool {
        stage
            == match self {
                TypedStage::ClientRequest(_) => Stages::ClientRequest,
                TypedStage::ClientResponse(_) => Stages::ClientResponse,
                TypedStage::OriginRequest(_) => Stages::OriginRequest,
                TypedStage::OriginResponse(_) => Stages::OriginResponse,
            }
    }
}

impl Verifier<SimpleProcessorsAnalysisContext, HostMetadataVerificationError>
    for PsMetadataAnalyzer
{
    fn analyze(
        &self,
        v: &gmd::spec::TypedGenericMetadata<()>,
        input: SimpleProcessorsAnalysisContext,
    ) -> AnalysisResult<SimpleProcessorsAnalysisContext, HostMetadataVerificationError> {
        let generic_stage = serde_json::from_value::<TypedGenericStage>(
            serde_json::to_value(v)
                .map_err(|e| HostMetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| HostMetadataVerificationError::JsonError(e.to_string()))?;

        let res = verify_ps_request_stage(&generic_stage, &input.scopes)
            .map_err(HostMetadataVerificationError::PsVerificationError)?;

        Ok((input, CdniVerificationKey::Stage(Box::new(res))))
    }

    fn type_name(&self, candidate: &str) -> bool {
        candidate == TypedClientRequestStage::<()>::typed_cdni_metadata_name()
            || candidate == TypedClientResponseStage::<()>::typed_cdni_metadata_name()
            || candidate == TypedOriginRequestStage::<()>::typed_cdni_metadata_name()
            || candidate == TypedOriginResponseStage::<()>::typed_cdni_metadata_name()
    }
}
