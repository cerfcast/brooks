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

//! The Source Generic Metadata Processing Implementation

use crate::cdni::{
    gmd::{
        self,
        spec::{Source, TypedSource},
    },
    gmdp::{InterpretationResult, Interpreter, ProcessedRequestResponse, Verifier},
    md::verify::CdniVerificationKey,
    mi::MetadataInformationResultElements,
    processors::{SimpleProcessorsAnalysisContext, SimpleProcessorsInterpreterContext},
    ps::interpret::PsInterpretValue,
};
use crate::{
    cdni::{
        gmdp::{AnalysisResult, Error, Stages},
        md::verify::HostMetadataVerificationError,
    },
    tools::prr::{self, Prr},
};

use std::fmt::Debug;
use std::str::FromStr;
use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone, Default)]
pub struct SourceVerificationKey {
    endpoints: Vec<SocketAddr>,
    protocol: SourceProtocol,
}

#[derive(Debug, Clone, Default)]
pub enum SourceProtocol {
    #[default]
    Http11,
}

impl Display for SourceProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceProtocol::Http11 => write!(f, "Http1/1"),
        }
    }
}

impl FromStr for SourceProtocol {
    type Err = HostMetadataVerificationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "http/1.1" {
            return Ok(SourceProtocol::Http11);
        }
        Err(HostMetadataVerificationError::JsonError(format!(
            "{} is not a valid protocol for Source MI",
            s
        )))
    }
}

#[derive(Debug)]
pub struct SourceMetadataAnalyzer {}

impl
    Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > for TypedSource<SourceVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (PsInterpretValue, MetadataInformationResultElements),
        Error,
    > {
        let reqres = &*input.rr;

        let req: reqwest::Request = reqres
            .try_into()
            .map_err(|e: prr::Error| Error::InvalidInput(e.into()))?;

        let clientb = reqwest::Client::builder();
        let client = clientb.build().map_err(|e| Error::RuntimeError(e.into()))?;

        let result: Result<ProcessedRequestResponse, Error> = input.runtime.block_on(async {
            let r = client
                .execute(req)
                .await
                .map_err(|e| Error::RuntimeError(e.into()))?;

            let mut result = ProcessedRequestResponse::new_response(r.url(), &http::Method::GET);

            result
                .set_status(&r.status().as_u16())
                .map_err(|_| Error::RuntimeError(prr::Error::BadStatus.into()))?;

            let b = r.bytes().await.map_err(|e| Error::RuntimeError(e.into()))?;
            result.obody = b.to_vec();
            Ok(result)
        });

        Ok((
            input.with_new_rr(Box::new(result?)),
            (
                PsInterpretValue::MatchNo,
                MetadataInformationResultElements::default(),
            ),
        ))
    }

    fn stage(&self, stage: Stages) -> bool {
        stage == Stages::Source
    }
}

impl Verifier<SimpleProcessorsAnalysisContext, HostMetadataVerificationError>
    for SourceMetadataAnalyzer
{
    fn analyze(
        &self,
        v: &gmd::spec::TypedGenericMetadata<()>,
        input: SimpleProcessorsAnalysisContext,
    ) -> AnalysisResult<SimpleProcessorsAnalysisContext, HostMetadataVerificationError> {
        let typed_source = serde_json::from_value::<TypedSource<()>>(
            serde_json::to_value(v)
                .map_err(|e| HostMetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| HostMetadataVerificationError::JsonError(e.to_string()))?;

        let typed_protocol: SourceProtocol = typed_source.value.protocol.parse()?;

        let mut typed_endpoints: Vec<SocketAddr> = vec![];
        for ep in typed_source.value.endpoints {
            typed_endpoints.push(ep.parse().map_err(|_| {
                HostMetadataVerificationError::JsonError(format!(
                    "{ep} is not a valid IP/Port combination"
                ))
            })?);
        }

        let res = TypedSource {
            tpe: TypedSource::<()>::typed_cdni_metadata_name(),
            value: Source {
                endpoints: vec![],
                protocol: SourceProtocol::Http11.to_string(),
                aug: SourceVerificationKey {
                    endpoints: typed_endpoints,
                    protocol: typed_protocol,
                },
            },
        };

        Ok((input, CdniVerificationKey::Source(Box::new(res))))
    }

    fn type_name(&self, candidate: &str) -> bool {
        candidate == TypedSource::<()>::typed_cdni_metadata_name()
    }
}
