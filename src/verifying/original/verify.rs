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
        original::spec::{SourceMetadata, TypedSourceMetadata},
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
    pub sources: Vec<(Vec<SocketAddr>, SourceProtocol)>,
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

fn analyze_source_metadata(
    to_verify: TypedSourceMetadata<()>,
    input: MetadataAnalysisContext,
) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
    let mut sources: Vec<(Vec<SocketAddr>, SourceProtocol)> = vec![];

    for s in &to_verify.value.sources {
        let typed_protocol: SourceProtocol = s.protocol.parse()?;

        let mut typed_endpoints: Vec<SocketAddr> = vec![];
        for ep in &s.endpoints {
            typed_endpoints.push(ep.parse().map_err(|_| {
                MetadataVerificationError::JsonError(format!(
                    "{ep} is not a valid IP/Port combination"
                ))
            })?);
        }

        sources.push((typed_endpoints, typed_protocol));
    }

    let res = TypedSourceMetadata {
        tpe: TypedSourceMetadata::<()>::typed_cdni_metadata_name(),
        value: SourceMetadata::<SourceVerificationKey> {
            sources: to_verify.value.sources,
            aug: SourceVerificationKey { sources },
        },
    };

    Ok((input, CdniVerificationKey::Source(Box::new(res))))
}
#[derive(Debug)]
pub struct SourceMetadataAnalyzer {}

impl Verifier<MetadataAnalysisContext, MetadataVerificationError> for SourceMetadataAnalyzer {
    fn analyze(
        &self,
        v: &TypedGenericMetadata<()>,
        input: MetadataAnalysisContext,
    ) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
        let typed_source = serde_json::from_value::<TypedSourceMetadata<()>>(
            serde_json::to_value(v)
                .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?;

        analyze_source_metadata(typed_source, input)
    }

    fn type_name(&self, candidate: &str) -> bool {
        candidate == TypedSourceMetadata::<()>::typed_cdni_metadata_name()
    }
}

#[cfg(test)]
mod test_verify_source_metadata {
    use std::{assert_matches, path::Path};

    use crate::{
        cdni::original::spec::{SourceMetadata, TypedSourceMetadata},
        tests::read_test_file,
        verifying::{
            original::verify::analyze_source_metadata,
            verify::{CdniVerificationKey, MetadataAnalysisContext},
        },
    };

    #[test]
    fn test_verify_8006_example() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/source/modified-8006-example.json",
        ));
        let cop_metadata = serde_json::from_str::<TypedSourceMetadata<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            cop_metadata.tpe,
            TypedSourceMetadata::<()>::typed_cdni_metadata_name()
        );

        let (_, result) = analyze_source_metadata(cop_metadata, MetadataAnalysisContext::default())
            .expect("Could not verify valid CDNI Source Metadata");

        let result = match result {
            CdniVerificationKey::Source(source) => source,
            _ => panic!("Incorrect CDNI Verification Key type returned by analyze_source_metadata"),
        };

        assert_matches!(
            *result,
            TypedSourceMetadata {
                tpe: _,
                value: SourceMetadata { sources, aug: _ }
            }
            if sources.len() == 2 &&
                sources[0].endpoints.len() == 2 &&
                sources[1].endpoints.len() == 1
        )
    }
}
