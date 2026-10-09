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

//! All Metadata Information except for what is defined in Processing Stages.

use crate::{
    cdni::{
        gmd::spec::TypedGenericMetadata,
        original::spec::{Source, TypedSource},
    },
    verifying::verify::{
        AnalysisResult, CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError,
        Verifier,
    },
};
use std::{
    fmt::{Debug, Display},
    net::SocketAddr,
    str::FromStr,
};

#[derive(Debug, Clone, Default)]
pub struct SourceVerificationKey {
    pub endpoints: Vec<SocketAddr>,
    pub protocol: SourceProtocol,
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
    type Err = MetadataVerificationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "http/1.1" {
            return Ok(SourceProtocol::Http11);
        }
        Err(MetadataVerificationError::JsonError(format!(
            "{} is not a valid protocol for Source MI",
            s
        )))
    }
}

#[derive(Debug)]
pub struct SourceMetadataAnalyzer {}

impl Verifier<MetadataAnalysisContext, MetadataVerificationError> for SourceMetadataAnalyzer {
    fn analyze(
        &self,
        v: &TypedGenericMetadata<()>,
        input: MetadataAnalysisContext,
    ) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
        let typed_source = serde_json::from_value::<TypedSource<()>>(
            serde_json::to_value(v)
                .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?;

        let typed_protocol: SourceProtocol = typed_source.value.protocol.parse()?;

        let mut typed_endpoints: Vec<SocketAddr> = vec![];
        for ep in typed_source.value.endpoints {
            typed_endpoints.push(ep.parse().map_err(|_| {
                MetadataVerificationError::JsonError(format!(
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
