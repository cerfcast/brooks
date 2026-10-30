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

//! CORS Verification

use regex::Regex;

use crate::{
    cdni::{
        cors::spec::{
            AccessControlAllowOrigin, AccessControlAllowOriginPattern, CrossoriginPolicy,
            TypedCrossoriginPolicy,
        },
        gmd::spec::TypedGenericMetadata,
    },
    verifying::verify::{
        AnalysisResult, CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError,
        Verifier,
    },
};
use std::{
    error::Error,
    fmt::{Debug, Display},
};

#[derive(Debug, Clone, Default)]
pub struct CrossoriginPolicyVerificationKey {}

#[derive(Debug)]
pub enum AccessControlAllowOriginVerificationError {
    BadPattern(Box<dyn Error>),
}

impl Display for AccessControlAllowOriginVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessControlAllowOriginVerificationError::BadPattern(error) => {
                write!(f, "Bad pattern in allow-origin: {error}")
            }
        }
    }
}

impl Error for AccessControlAllowOriginVerificationError {}

#[derive(Debug)]
pub enum CrossoriginPolicyVerificationError {
    AccessControlAllowOriginVerificationError(AccessControlAllowOriginVerificationError),
}

impl Display for CrossoriginPolicyVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrossoriginPolicyVerificationError::AccessControlAllowOriginVerificationError(
                access_control_allow_origin_verification_error,
            ) => write!(f, "{access_control_allow_origin_verification_error}"),
        }
    }
}

impl Error for CrossoriginPolicyVerificationError {}

fn verify_allow_origin(
    allow_origin: &AccessControlAllowOrigin<()>,
) -> Result<AccessControlAllowOrigin<CrossoriginPolicyVerificationKey>, MetadataVerificationError> {
    let mut analyzed_allow_origin: Vec<AccessControlAllowOriginPattern> = vec![];
    for pattern in &allow_origin.allow_origin {
        let r = Regex::new(&pattern.pattern).map_err(|e| {
            MetadataVerificationError::CrossoriginPolicyVerificationError(
                CrossoriginPolicyVerificationError::AccessControlAllowOriginVerificationError(
                    AccessControlAllowOriginVerificationError::BadPattern(e.into()),
                )
                .into(),
            )
        })?;
        analyzed_allow_origin.push(AccessControlAllowOriginPattern {
            pattern: pattern.pattern.clone(),
            aug: Some(r),
        });
    }

    Ok(AccessControlAllowOrigin {
        allow_origin: analyzed_allow_origin,
        wildcard_return: allow_origin.wildcard_return,
        aug: CrossoriginPolicyVerificationKey::default(),
    })
}

#[derive(Debug)]
pub struct CrossoriginPolicyMetadataAnalyzer {}

fn analyze_crossoriginpolicy(
    typed_cop: &TypedCrossoriginPolicy<()>,
    input: MetadataAnalysisContext,
) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
    let verified_allow_origin = verify_allow_origin(&typed_cop.value.allow_origin)?;

    let res = TypedCrossoriginPolicy::<CrossoriginPolicyVerificationKey> {
        tpe: TypedCrossoriginPolicy::<()>::typed_cdni_metadata_name(),
        value: CrossoriginPolicy::<CrossoriginPolicyVerificationKey> {
            allow_origin: verified_allow_origin,
            preflight_only: typed_cop.value.preflight_only,
            aug: CrossoriginPolicyVerificationKey::default(),
        },
    };

    Ok((input, CdniVerificationKey::CrossoriginPolicy(Box::new(res))))
}

impl Verifier<MetadataAnalysisContext, MetadataVerificationError>
    for CrossoriginPolicyMetadataAnalyzer
{
    fn analyze(
        &self,
        v: &TypedGenericMetadata<()>,
        input: MetadataAnalysisContext,
    ) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
        let typed_cop = serde_json::from_value::<TypedCrossoriginPolicy<()>>(
            serde_json::to_value(v)
                .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?;

        // Verify the allow-list.
        analyze_crossoriginpolicy(&typed_cop, input)
    }

    fn type_name(&self, candidate: &str) -> bool {
        candidate == TypedCrossoriginPolicy::<()>::typed_cdni_metadata_name()
    }
}

#[cfg(test)]
mod test {
    use std::{assert_matches, path::Path};

    use crate::{
        cdni::cors::spec::{CrossoriginPolicy, TypedCrossoriginPolicy},
        environment::scope::Scopes,
        mel::types::Type,
        tests::read_test_file,
        verifying::{
            cors::verify::analyze_crossoriginpolicy,
            verify::{CdniVerificationKey, MetadataAnalysisContext},
        },
    };

    #[test]
    fn test_verify_draft_figure1() {
        let json = read_test_file(Path::new("./src/cdni/tests/cors/figure1.json"));
        let cop_metadata = serde_json::from_str::<TypedCrossoriginPolicy<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            cop_metadata.tpe,
            TypedCrossoriginPolicy::<()>::typed_cdni_metadata_name()
        );

        let (_, result) = analyze_crossoriginpolicy(
            &cop_metadata,
            MetadataAnalysisContext {
                scopes: Scopes::<Type>::default(),
            },
        )
        .expect("Could not verify valid MI.CrossoriginPolicy metadata");

        let result = match result {
            CdniVerificationKey::CrossoriginPolicy(r) => r,
            _ => todo!(),
        };

        assert_matches!(
            *result,
            TypedCrossoriginPolicy {
                tpe: _,
                value: CrossoriginPolicy {
                    allow_origin: ao,
                    preflight_only: None,
                    aug: _,
                }
            } if ao.allow_origin.len() == 1
        );
    }

    #[test]
    fn test_verify_bad_pattern() {
        let json = read_test_file(Path::new("./src/cdni/tests/cors/bad_pattern.json"));
        let cop_metadata = serde_json::from_str::<TypedCrossoriginPolicy<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            cop_metadata.tpe,
            TypedCrossoriginPolicy::<()>::typed_cdni_metadata_name()
        );

        let result = analyze_crossoriginpolicy(
            &cop_metadata,
            MetadataAnalysisContext {
                scopes: Scopes::<Type>::default(),
            },
        );

        assert_matches!(result, Err(_));
    }
}
