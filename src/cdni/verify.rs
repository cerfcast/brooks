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

//! Verification of CDNI JSON

use serde::Serialize;

use crate::{
    cdni::{
        spec::{
            ExpressionMatch, ProcessingStages, StageMetadata, StageRules, TypedExpressionMatch,
            TypedGenericMetadata, TypedHeader, TypedHeaderTransform, TypedProcessingStages,
            TypedRequestTransform, TypedResponseTransform, TypedStageMetadata, TypedStageRules,
            TypedSyntheticResponse,
        },
        visit::CdniVisitor,
    },
    mel::{
        analysis::{Analyzed, MelAnalysisLocatableError, analyze},
        ast::Expr,
        compiler::{self, compile::CompilerError},
        scope::Scopes,
        tvs::Type,
    },
};

use std::fmt::Debug;

type CdniVisitorResult<T, E> = Result<T, E>;

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationError {
    #[default]
    NoError,
    WrongType,
    WrongGenericMetadataTypeName(String, String),
    NoVerifiedValue,
    ExpressionCompile(CompilerError),
    ExpressionAnalyze(MelAnalysisLocatableError),
    ExpressionWrongType(Type, Type),
}

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationKey {
    Expr(Expr<Analyzed>),
    #[default]
    None,
}

impl Serialize for CdniVerificationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

#[derive(Debug, Clone, Default)]
struct CdniVerifier {}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum CdniVerifierContextValue {
    ProcessingStages(TypedProcessingStages<CdniVerificationKey>),
    StageRules(TypedStageRules<CdniVerificationKey>),
    ExpressionMatch(TypedExpressionMatch<CdniVerificationKey>),
    StageMetadata(TypedStageMetadata<CdniVerificationKey>),
    GenericMetadata(TypedGenericMetadata<CdniVerificationKey>),
    RequestTransform(TypedRequestTransform<CdniVerificationKey>),
    ResponseTransform(TypedResponseTransform<CdniVerificationKey>),
}

#[derive(Debug, Clone, Default)]
struct CdniVerifierContext {
    value: Option<CdniVerifierContextValue>,
}

type VerifiedCdni = ProcessingStages<CdniVerificationKey>;

macro_rules! expect_maybe_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => Some(v),
            None => None,
            _ => return Err(CdniVerificationError::WrongType),
        }
    };
}

macro_rules! expect_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => v,
            None => return Err(CdniVerificationError::NoVerifiedValue),
            _ => return Err(CdniVerificationError::WrongType),
        }
    };
}

macro_rules! make_context_value {
    ($value:expr, $vt:path, $a:ident, $tvt:ident) => {
        Some($vt($tvt::<$a>::typed_value($value)))
    };
}

macro_rules! check_generic_md_typename {
    ($value:expr, $tn:ident) => {
        if $value.tpe != $tn::<()>::typed_generic_metadata_name() {
            return Err(CdniVerificationError::WrongGenericMetadataTypeName(
                $tn::<()>::typed_generic_metadata_name(),
                $value.tpe.clone(),
            ));
        }
    };
}

impl CdniVisitor<(), CdniVerifierContext, CdniVerificationError> for CdniVerifier {
    fn visit_processing_stages(
        &self,
        v: &TypedProcessingStages<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedProcessingStages);

        let mut result = ProcessingStages::<CdniVerificationKey>::default();
        let mut rc = c.clone();

        let v = &v.value;

        for csr in &v.client_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.client_req.push(tsr.clone());
            };
        }

        for csr in &v.client_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.client_res.push(tsr.clone());
            };
        }

        for csr in &v.origin_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.origin_req.push(tsr.clone());
            };
        }

        for csr in &v.origin_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.origin_res.push(tsr.clone());
            };
        }

        rc.value = make_context_value!(
            result,
            CdniVerifierContextValue::ProcessingStages,
            CdniVerificationKey,
            TypedProcessingStages
        );

        Ok(rc.clone())
    }

    fn visit_stage_rules(
        &self,
        v: &TypedStageRules<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        // Check whether the expression has the right type!

        check_generic_md_typename!(v, TypedStageRules);

        let v = &v.value;

        let mut result = if let Some(mtch) = &v.mtch {
            let expr = self.visit_expression_match(mtch, &c.clone())?;

            let expr =
                expect_maybe_some_value!(&expr.value, CdniVerifierContextValue::ExpressionMatch);
            StageRules::<CdniVerificationKey> {
                mtch: expr.cloned(),
                stage_metadata: TypedStageMetadata::default(),
                aug: CdniVerificationKey::None,
            }
        } else {
            StageRules::<CdniVerificationKey> {
                mtch: None,
                stage_metadata: TypedStageMetadata::default(),
                aug: CdniVerificationKey::None,
            }
        };

        result.stage_metadata = expect_some_value!(
            &self
                .visit_stage_metadata(&v.stage_metadata, &c.clone())?
                .value,
            CdniVerifierContextValue::StageMetadata
        )
        .clone();

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::StageRules,
                CdniVerificationKey,
                TypedStageRules
            ),
        })
    }

    fn visit_expression_match(
        &self,
        v: &TypedExpressionMatch<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedExpressionMatch);

        let v = &v.value;

        let expr =
            compiler::compile(&v.expression).map_err(CdniVerificationError::ExpressionCompile)?;
        let expr =
            analyze(&expr, Scopes::default()).map_err(CdniVerificationError::ExpressionAnalyze)?;

        if expr.tipe() != Type::Boolean {
            return Err(CdniVerificationError::ExpressionWrongType(
                Type::Boolean,
                expr.tipe(),
            ));
        }
        Ok(CdniVerifierContext {
            value: make_context_value!(
                ExpressionMatch {
                    expression: v.expression.clone(),
                    aug: CdniVerificationKey::Expr(expr),
                },
                CdniVerifierContextValue::ExpressionMatch,
                CdniVerificationKey,
                TypedExpressionMatch
            ),
        })
    }

    fn visit_stage_metadata(
        &self,
        v: &TypedStageMetadata<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedStageMetadata);

        let v = &v.value;

        let mut result = if let Some(generic) = &v.generic {
            let mut result: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];
            for generics in generic {
                let generic = expect_some_value!(
                    self.visit_generic_metadata(generics, &c.clone())?.value,
                    CdniVerifierContextValue::GenericMetadata
                )
                .clone();
                result.push(generic);
            }

            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: Some(result),
                aug: CdniVerificationKey::None,
            }
        } else {
            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: None,
                aug: CdniVerificationKey::None,
            }
        };

        result.request_xform = if let Some(reqt) = &v.request_xform {
            Some(
                expect_some_value!(
                    self.visit_request_transform(reqt, &c.clone())?.value,
                    CdniVerifierContextValue::RequestTransform
                )
                .clone(),
            )
        } else {
            None
        };

        result.response_xform = if let Some(reqt) = &v.request_xform {
            Some(
                expect_some_value!(
                    self.visit_response_transform(reqt, &c.clone())?.value,
                    CdniVerifierContextValue::ResponseTransform
                )
                .clone(),
            )
        } else {
            None
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::StageMetadata,
                CdniVerificationKey,
                TypedStageMetadata
            ),
        })
    }

    fn visit_request_transform(
        &self,
        _v: &TypedRequestTransform<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
    }

    fn visit_response_transform(
        &self,
        _v: &TypedRequestTransform<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
    }

    fn visit_generic_metadata(
        &self,
        v: &TypedGenericMetadata<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        // We only verify that the type starts with "MI.".
        if !v.tpe.starts_with("MI.") {
            return Err(CdniVerificationError::WrongGenericMetadataTypeName(
                "MI. ...".to_string(),
                v.tpe.clone(),
            ));
        }

        Ok(CdniVerifierContext {
            value: Some(CdniVerifierContextValue::GenericMetadata(
                TypedGenericMetadata {
                    tpe: v.tpe.clone(),
                    value: v.value.clone(),
                    aug: CdniVerificationKey::None,
                },
            )),
        })
    }

    fn visit_header_transform(
        &self,
        _v: &TypedHeaderTransform<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
    }

    fn visit_header(
        &self,
        _v: &TypedHeader<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
    }

    fn visit_synthetic_response(
        &self,
        _v: &TypedSyntheticResponse<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
    }
}

/// Verify the (semantic) validity of some CDNI JSON
///
/// In particular, test whether
/// 1. the MEL expressions have the proper types, and
/// 2. the generic metadata type names are correct.
///
#[allow(clippy::result_large_err)]
pub fn verify_cdni(
    stages: &TypedProcessingStages<()>,
) -> Result<TypedProcessingStages<CdniVerificationKey>, CdniVerificationError> {
    let verifier = CdniVerifier {};
    let context = CdniVerifierContext::default();
    let value = &stages;

    let result = verifier.visit_processing_stages(value, &context)?;

    Ok(expect_some_value!(&result.value, CdniVerifierContextValue::ProcessingStages).clone())
}

#[cfg(test)]
mod test_verify {
    use std::assert_matches;

    use crate::cdni::{
        spec::TypedProcessingStages,
        verify::{
            CdniVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
            verify_cdni,
        },
    };
    use crate::mel::tvs::Type;

    use std::path::Path;

    use crate::cdni::tests::test_helpers::read_test_file;

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let json = read_test_file(Path::new("./src/cdni/tests/example8-bad-typename.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with bad generic metadata typenames");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_simple() {
        let json = read_test_file(Path::new("./src/cdni/tests/simple_deserialize_verify.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify proper CDNI");

        let result_serialized =
            serde_json::to_string_pretty(&result).expect("Could not serialize verified CDNI");
        pretty_assertions::assert_eq!(json, result_serialized);
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/simple_deserialize_verify_wrong_match_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect_err("Could verify mistyped CDNI");

        assert_matches!(result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_stage_metadata_with_generic_metadata_bad_type_name() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/simple_deserialize_stage_metadata_with_generic_metadata_bad_type_name.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect_err("Could verify mistyped CDNI");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }
}
