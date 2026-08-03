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

//! Verification of CDNI Processing Stages JSON

use serde::Serialize;

use crate::{
    cdni::spec::TypedGenericMetadata,
    mel::{
        analysis::{Analyzed, MelAnalysisLocatableError, analyze},
        ast::Expr,
        compiler::{self, compile::MelCompilerLocatableError},
        scope::Scopes,
        tvs::Type,
    },
    ps::{
        spec::{
            ClientRequestStage, ClientResponseStage, ExpressionMatch, Header, HeaderTransform,
            MatchGroup, OriginRequestStage, OriginResponseStage, ProcessingStages,
            RequestTransform, ResponseTransform, StageMetadata, StageRules, SyntheticResponse,
            TypedClientRequestStage, TypedClientResponseStage, TypedExpressionMatch,
            TypedGenericStage, TypedHeader, TypedHeaderTransform, TypedMatchGroup,
            TypedOriginRequestStage, TypedOriginResponseStage, TypedProcessingStages,
            TypedRequestTransform, TypedResponseTransform, TypedStage, TypedStageMetadata,
            TypedStageRules, TypedSyntheticResponse,
        },
        verify::PsVerificationError::ParseError,
        visit::PsVisitor,
    },
};

use std::fmt::{Debug, Display};

type PsVisitorResult<T, E> = Result<T, E>;

#[derive(Debug, Clone, Default)]
pub enum PsVerificationError {
    #[default]
    NoError,
    WrongType,
    WrongGenericMetadataTypeName(String, String),
    NoVerifiedValue,
    ExpressionCompile(Box<MelCompilerLocatableError>),
    ExpressionAnalyze(Box<MelAnalysisLocatableError>),
    ExpressionWrongType(Type, Type),
    InvalidTypedResponseStage,
    ParseError,
}

impl Display for PsVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsVerificationError::NoError => write!(f, "No error!"),
            PsVerificationError::WrongType => write!(f, "Wrong type"),
            PsVerificationError::WrongGenericMetadataTypeName(expected, actual) => write!(
                f,
                "Wrong generic metadata type; expected {expected} and got {actual}"
            ),
            PsVerificationError::NoVerifiedValue => write!(f, "Missing verified value"),
            PsVerificationError::ExpressionCompile(compiler_error) => {
                write!(f, "Expression compilation error: {compiler_error}")
            }
            PsVerificationError::ExpressionAnalyze(mel_analysis_locatable_error) => write!(
                f,
                "Expression analysis error: {mel_analysis_locatable_error}"
            ),
            PsVerificationError::ExpressionWrongType(expected, actual) => write!(
                f,
                "Wrong expression type; expected {expected} and got {actual}"
            ),
            PsVerificationError::InvalidTypedResponseStage => {
                write!(f, "Invalid typed response stage")
            }
            ParseError => write!(f, "Parse error"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum PsVerificationKey {
    Expr(Expr<Analyzed>),
    ExprPair(Option<Expr<Analyzed>>, Option<Expr<Analyzed>>),
    #[default]
    None,
}

impl Serialize for PsVerificationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

impl TypedGenericStage {
    pub fn typed(&self) -> Result<TypedStage<()>, Box<PsVerificationError>> {
        if self.tpe == TypedClientRequestStage::<()>::typed_generic_metadata_name() {
            let stages = serde_json::from_str::<TypedClientRequestStage<()>>(
                &(serde_json::to_string(self).map_err(|_| ParseError)?),
            )
            .map_err(|_| ParseError)?;
            Ok(TypedStage::ClientRequest(stages))
        } else if self.tpe == TypedClientResponseStage::<()>::typed_generic_metadata_name() {
            let stages = serde_json::from_str::<TypedClientResponseStage<()>>(
                &(serde_json::to_string(self).map_err(|_| ParseError)?),
            )
            .map_err(|_| ParseError)?;
            Ok(TypedStage::ClientResponse(stages))
        } else if self.tpe == TypedOriginRequestStage::<()>::typed_generic_metadata_name() {
            let stages = serde_json::from_str::<TypedOriginRequestStage<()>>(
                &(serde_json::to_string(self).map_err(|_| ParseError)?),
            )
            .map_err(|_| ParseError)?;
            Ok(TypedStage::OriginRequest(stages))
        } else if self.tpe == TypedOriginResponseStage::<()>::typed_generic_metadata_name() {
            let stages = serde_json::from_str::<TypedOriginResponseStage<()>>(
                &(serde_json::to_string(self).map_err(|_| ParseError)?),
            )
            .map_err(|_| ParseError)?;
            Ok(TypedStage::OriginResponse(stages))
        } else {
            Err(PsVerificationError::InvalidTypedResponseStage.into())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PsVerifier {}

impl PsVerifier {
    pub fn compile_and_analyze_expr(
        source: &str,
        scopes: &Scopes<Type>,
    ) -> Result<Expr<Analyzed>, Box<PsVerificationError>> {
        let expr = compiler::compile(source)
            .map_err(|e| Box::new(PsVerificationError::ExpressionCompile(Box::new(e))))?;
        analyze(&expr, scopes).map_err(|e| Box::new(PsVerificationError::ExpressionAnalyze(e)))
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub(crate) enum PsVerifierContextValue {
    ProcessingStages(TypedProcessingStages<PsVerificationKey>),
    StageRules(TypedStageRules<PsVerificationKey>),
    ExpressionMatch(TypedExpressionMatch<PsVerificationKey>),
    StageMetadata(TypedStageMetadata<PsVerificationKey>),
    GenericMetadata(TypedGenericMetadata<PsVerificationKey>),
    RequestTransform(TypedRequestTransform<PsVerificationKey>),
    ResponseTransform(TypedResponseTransform<PsVerificationKey>),
    HeaderTransform(TypedHeaderTransform<PsVerificationKey>),
    Header(TypedHeader<PsVerificationKey>),
    SyntheticResponse(TypedSyntheticResponse<PsVerificationKey>),
    MatchGroup(TypedMatchGroup<PsVerificationKey>),
    ClientRequestStage(TypedClientRequestStage<PsVerificationKey>),
    ClientResponseStage(TypedClientResponseStage<PsVerificationKey>),
    OriginRequestStage(TypedOriginRequestStage<PsVerificationKey>),
    OriginResponseStage(TypedOriginResponseStage<PsVerificationKey>),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PsVerifierContext {
    pub value: Option<PsVerifierContextValue>,
    pub scopes: Scopes<Type>,
}

type VerifiedPs = ProcessingStages<PsVerificationKey>;

macro_rules! expect_maybe_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => Some(v),
            None => None,
            _ => return Err(Box::new(PsVerificationError::WrongType)),
        }
    };
}

macro_rules! expect_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => v,
            None => return Err(Box::new(PsVerificationError::NoVerifiedValue)),
            _ => return Err(Box::new(PsVerificationError::WrongType)),
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
            return Err(Box::new(PsVerificationError::WrongGenericMetadataTypeName(
                $tn::<()>::typed_generic_metadata_name(),
                $value.tpe.clone(),
            )));
        }
    };
}

impl PsVisitor<(), PsVerifierContext, Box<PsVerificationError>> for PsVerifier {
    fn visit_processing_stages(
        &mut self,
        v: &TypedProcessingStages<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedProcessingStages);

        let mut result = ProcessingStages::<PsVerificationKey>::default();
        let mut rc = c.clone();

        let v = &v.value;

        for csr in &v.client_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, PsVerifierContextValue::StageRules)
            {
                result.client_req.push(tsr.clone());
            };
        }

        for csr in &v.client_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, PsVerifierContextValue::StageRules)
            {
                result.client_res.push(tsr.clone());
            };
        }

        for csr in &v.origin_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, PsVerifierContextValue::StageRules)
            {
                result.origin_req.push(tsr.clone());
            };
        }

        for csr in &v.origin_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, PsVerifierContextValue::StageRules)
            {
                result.origin_res.push(tsr.clone());
            };
        }

        rc.value = make_context_value!(
            result,
            PsVerifierContextValue::ProcessingStages,
            PsVerificationKey,
            TypedProcessingStages
        );

        Ok(rc.clone())
    }

    fn visit_stage_rules(
        &mut self,
        v: &TypedStageRules<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        // Check whether the expression has the right type!

        check_generic_md_typename!(v, TypedStageRules);

        let v = &v.value;

        let mut result = if let Some(mtch) = &v.mtch {
            let expr = self.visit_expression_match(mtch, &c.clone())?;

            let expr =
                expect_maybe_some_value!(&expr.value, PsVerifierContextValue::ExpressionMatch);
            StageRules::<PsVerificationKey> {
                mtch: expr.cloned(),
                stage_metadata: TypedStageMetadata::default(),
                aug: PsVerificationKey::None,
            }
        } else {
            StageRules::<PsVerificationKey> {
                mtch: None,
                stage_metadata: TypedStageMetadata::default(),
                aug: PsVerificationKey::None,
            }
        };

        result.stage_metadata = expect_some_value!(
            &self
                .visit_stage_metadata(&v.stage_metadata, &c.clone())?
                .value,
            PsVerifierContextValue::StageMetadata
        )
        .clone();

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::StageRules,
                PsVerificationKey,
                TypedStageRules
            ),
        })
    }

    fn visit_expression_match(
        &mut self,
        v: &TypedExpressionMatch<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedExpressionMatch);

        let v = &v.value;

        let expr = Self::compile_and_analyze_expr(&v.expression, &c.scopes)?;

        if expr.tipe() != Type::Boolean {
            return Err(Box::new(PsVerificationError::ExpressionWrongType(
                Type::Boolean,
                expr.tipe(),
            )));
        }
        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                ExpressionMatch {
                    expression: v.expression.clone(),
                    aug: PsVerificationKey::Expr(expr),
                },
                PsVerifierContextValue::ExpressionMatch,
                PsVerificationKey,
                TypedExpressionMatch
            ),
        })
    }

    fn visit_stage_metadata(
        &mut self,
        v: &TypedStageMetadata<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedStageMetadata);

        let v = &v.value;

        let mut result = if let Some(generic) = &v.generic {
            let mut result: Vec<TypedGenericMetadata<PsVerificationKey>> = vec![];
            for generics in generic {
                let generic = expect_some_value!(
                    self.visit_generic_metadata(generics, &c.clone())?.value,
                    PsVerifierContextValue::GenericMetadata
                )
                .clone();
                result.push(generic);
            }

            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: Some(result),
                aug: PsVerificationKey::None,
            }
        } else {
            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: None,
                aug: PsVerificationKey::None,
            }
        };

        result.request_xform = if let Some(reqt) = &v.request_xform {
            Some(
                expect_some_value!(
                    self.visit_request_transform(reqt, &c.clone())?.value,
                    PsVerifierContextValue::RequestTransform
                )
                .clone(),
            )
        } else {
            None
        };

        result.response_xform = if let Some(reqt) = &v.response_xform {
            Some(
                expect_some_value!(
                    self.visit_response_transform(reqt, &c.clone())?.value,
                    PsVerifierContextValue::ResponseTransform
                )
                .clone(),
            )
        } else {
            None
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::StageMetadata,
                PsVerificationKey,
                TypedStageMetadata
            ),
        })
    }

    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedRequestTransform);

        let v = &v.value;

        let mut result = if let Some(header_xform) = &v.xform {
            let header = Some(
                expect_some_value!(
                    self.visit_header_transform(header_xform, &c.clone())?.value,
                    PsVerifierContextValue::HeaderTransform
                )
                .clone(),
            );

            RequestTransform {
                xform: header,
                uri: v.uri.clone(),
                uri_is_expr: v.uri_is_expr,
                aug: PsVerificationKey::None,
            }
        } else {
            RequestTransform {
                xform: None,
                uri: v.uri.clone(),
                uri_is_expr: v.uri_is_expr,
                aug: PsVerificationKey::None,
            }
        };

        result.aug = if let Some(uri) = &v.uri {
            if let Some(uri_is_expr) = &v.uri_is_expr
                && *uri_is_expr
            {
                let expr = Self::compile_and_analyze_expr(uri, &c.scopes)?;

                if expr.tipe() != Type::String {
                    return Err(Box::new(PsVerificationError::ExpressionWrongType(
                        Type::String,
                        expr.tipe(),
                    )));
                }
                PsVerificationKey::Expr(expr)
            } else {
                PsVerificationKey::None
            }
        } else {
            PsVerificationKey::None
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::RequestTransform,
                PsVerificationKey,
                TypedRequestTransform
            ),
        })
    }

    fn visit_response_transform(
        &mut self,
        v: &TypedResponseTransform<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedResponseTransform);

        let v = &v.value;

        let mut result = if let Some(header_xform) = &v.xform {
            let header = Some(
                expect_some_value!(
                    self.visit_header_transform(header_xform, &c.clone())?.value,
                    PsVerifierContextValue::HeaderTransform
                )
                .clone(),
            );

            ResponseTransform {
                xform: header,
                response_status: v.response_status.clone(),
                response_status_expr: v.response_status_expr,
                synthetic: None,
                aug: PsVerificationKey::None,
            }
        } else {
            ResponseTransform {
                xform: None,
                response_status: v.response_status.clone(),
                response_status_expr: v.response_status_expr,
                synthetic: None,
                aug: PsVerificationKey::None,
            }
        };

        result.synthetic = if let Some(synthetic) = &v.synthetic {
            Some(
                expect_some_value!(
                    self.visit_synthetic_response(synthetic, &c.clone())?.value,
                    PsVerifierContextValue::SyntheticResponse
                )
                .clone(),
            )
        } else {
            None
        };

        result.aug = if let Some(rs) = &v.response_status {
            if let Some(rs_is_expr) = &v.response_status_expr
                && *rs_is_expr
            {
                let expr = Self::compile_and_analyze_expr(rs, &c.scopes)?;

                if expr.tipe() != Type::Integer {
                    return Err(Box::new(PsVerificationError::ExpressionWrongType(
                        Type::Integer,
                        expr.tipe(),
                    )));
                }
                PsVerificationKey::Expr(expr)
            } else {
                PsVerificationKey::None
            }
        } else {
            PsVerificationKey::None
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::ResponseTransform,
                PsVerificationKey,
                TypedResponseTransform
            ),
        })
    }

    fn visit_generic_metadata(
        &mut self,
        v: &TypedGenericMetadata<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        // We only verify that the type starts with "MI.".
        if !v.tpe.starts_with("MI.") {
            return Err(Box::new(PsVerificationError::WrongGenericMetadataTypeName(
                "MI. ...".to_string(),
                v.tpe.clone(),
            )));
        }

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: Some(PsVerifierContextValue::GenericMetadata(
                TypedGenericMetadata {
                    tpe: v.tpe.clone(),
                    value: v.value.clone(),
                    aug: PsVerificationKey::None,
                },
            )),
        })
    }

    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedHeaderTransform);
        let v = &v.value;

        let mut result = HeaderTransform {
            delete: v.delete.clone(),
            ..Default::default()
        };

        result.add = if let Some(adds) = &v.add {
            let mut verified_adds: Vec<TypedHeader<PsVerificationKey>> = vec![];
            for add in adds {
                verified_adds.push(
                    expect_some_value!(
                        self.visit_header(add, &c.clone())?.value,
                        PsVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_adds)
        } else {
            None
        };

        result.replace = if let Some(replaceds) = &v.replace {
            let mut verified_replaceds: Vec<TypedHeader<PsVerificationKey>> = vec![];
            for replaced in replaceds {
                verified_replaceds.push(
                    expect_some_value!(
                        self.visit_header(replaced, &c.clone())?.value,
                        PsVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_replaceds)
        } else {
            None
        };
        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::HeaderTransform,
                PsVerificationKey,
                TypedHeaderTransform
            ),
        })
    }

    fn visit_header(
        &mut self,
        v: &TypedHeader<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedHeader);

        let v = &v.value;
        let mut result = Header::<PsVerificationKey> {
            name: v.name.clone(),
            value: v.value.clone(),
            value_expr: v.value_expr,
            aug: PsVerificationKey::None,
        };

        result.aug = if let Some(hv_is_expr) = &v.value_expr
            && *hv_is_expr
        {
            let expr = Self::compile_and_analyze_expr(&v.value, &c.scopes)?;

            if expr.tipe() != Type::String {
                return Err(Box::new(PsVerificationError::ExpressionWrongType(
                    Type::String,
                    expr.tipe(),
                )));
            }
            PsVerificationKey::Expr(expr)
        } else {
            PsVerificationKey::None
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::Header,
                PsVerificationKey,
                TypedHeader
            ),
        })
    }

    fn visit_synthetic_response(
        &mut self,
        v: &TypedSyntheticResponse<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedSyntheticResponse);
        let v = &v.value;

        let mut result = SyntheticResponse {
            headers: None,
            response_status: v.response_status.clone(),
            response_status_expr: v.response_status_expr,
            body: v.body.clone(),
            body_expr: v.body_expr,
            aug: PsVerificationKey::None,
        };

        result.headers = if let Some(headers) = &v.headers {
            let mut verified_headers: Vec<TypedHeader<PsVerificationKey>> = vec![];
            for header in headers {
                verified_headers.push(
                    expect_some_value!(
                        self.visit_header(header, &c.clone())?.value,
                        PsVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_headers)
        } else {
            None
        };

        let response_status_expr = if let Some(rs) = &v.response_status {
            if let Some(rs_is_expr) = &v.response_status_expr
                && *rs_is_expr
            {
                let expr = Self::compile_and_analyze_expr(rs, &c.scopes)?;

                if expr.tipe() != Type::Integer {
                    return Err(Box::new(PsVerificationError::ExpressionWrongType(
                        Type::Integer,
                        expr.tipe(),
                    )));
                }
                Some(expr)
            } else {
                None
            }
        } else {
            None
        };

        let body_expr = if let Some(body) = &v.body {
            if let Some(body_is_expr) = &v.body_expr
                && *body_is_expr
            {
                let expr = Self::compile_and_analyze_expr(body, &c.scopes)?;

                if expr.tipe() != Type::String {
                    return Err(Box::new(PsVerificationError::ExpressionWrongType(
                        Type::String,
                        expr.tipe(),
                    )));
                }
                Some(expr)
            } else {
                None
            }
        } else {
            None
        };

        result.aug = PsVerificationKey::ExprPair(response_status_expr, body_expr);

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::SyntheticResponse,
                PsVerificationKey,
                TypedSyntheticResponse
            ),
        })
    }

    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedClientRequestStage);
        let v = &v.value;

        // Verify each of the match groups!
        let mut verified_mgs: Vec<TypedMatchGroup<PsVerificationKey>> = vec![];
        for mg in &v.match_groups {
            verified_mgs.push(
                expect_some_value!(
                    self.visit_match_group(mg, &c.clone())?.value,
                    PsVerifierContextValue::MatchGroup
                )
                .clone(),
            );
        }

        let result = ClientRequestStage {
            match_groups: verified_mgs,
            aug: PsVerificationKey::None,
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::ClientRequestStage,
                PsVerificationKey,
                TypedClientRequestStage
            ),
        })
    }

    fn visit_match_group(
        &mut self,
        v: &TypedMatchGroup<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedMatchGroup);
        let v = &v.value;

        let verified_if_rule = expect_some_value!(
            self.visit_stage_rules(&v.if_rule, &c.clone())?.value,
            PsVerifierContextValue::StageRules
        )
        .clone();

        let result = if let Some(else_if_rules) = &v.else_ifs {
            // Verify each of the match groups!
            let mut verified_eirs: Vec<TypedStageRules<PsVerificationKey>> = vec![];
            for eir in else_if_rules {
                verified_eirs.push(
                    expect_some_value!(
                        self.visit_stage_rules(eir, &c.clone())?.value,
                        PsVerifierContextValue::StageRules
                    )
                    .clone(),
                );
            }
            MatchGroup {
                if_rule: verified_if_rule,
                else_ifs: Some(verified_eirs),
                aug: PsVerificationKey::None,
            }
        } else {
            MatchGroup {
                if_rule: verified_if_rule,
                else_ifs: None,
                aug: PsVerificationKey::None,
            }
        };
        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::MatchGroup,
                PsVerificationKey,
                TypedMatchGroup
            ),
        })
    }

    fn visit_origin_request_stage(
        &mut self,
        v: &TypedOriginRequestStage<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedOriginRequestStage);
        let v = &v.value;

        // Verify each of the match groups!
        let mut verified_mgs: Vec<TypedMatchGroup<PsVerificationKey>> = vec![];
        for mg in &v.match_groups {
            verified_mgs.push(
                expect_some_value!(
                    self.visit_match_group(mg, &c.clone())?.value,
                    PsVerifierContextValue::MatchGroup
                )
                .clone(),
            );
        }

        let result = OriginRequestStage {
            match_groups: verified_mgs,
            aug: PsVerificationKey::None,
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::OriginRequestStage,
                PsVerificationKey,
                TypedOriginRequestStage
            ),
        })
    }

    fn visit_client_response_stage(
        &mut self,
        v: &TypedClientResponseStage<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedClientResponseStage);
        let v = &v.value;

        // Verify each of the match groups!
        let mut verified_mgs: Vec<TypedMatchGroup<PsVerificationKey>> = vec![];
        for mg in &v.match_groups {
            verified_mgs.push(
                expect_some_value!(
                    self.visit_match_group(mg, &c.clone())?.value,
                    PsVerifierContextValue::MatchGroup
                )
                .clone(),
            );
        }

        let result = ClientResponseStage {
            match_groups: verified_mgs,
            aug: PsVerificationKey::None,
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::ClientResponseStage,
                PsVerificationKey,
                TypedClientResponseStage
            ),
        })
    }

    fn visit_origin_response_stage(
        &mut self,
        v: &TypedOriginResponseStage<()>,
        c: &PsVerifierContext,
    ) -> PsVisitorResult<PsVerifierContext, Box<PsVerificationError>> {
        check_generic_md_typename!(v, TypedOriginResponseStage);
        let v = &v.value;

        // Verify each of the match groups!
        let mut verified_mgs: Vec<TypedMatchGroup<PsVerificationKey>> = vec![];
        for mg in &v.match_groups {
            verified_mgs.push(
                expect_some_value!(
                    self.visit_match_group(mg, &c.clone())?.value,
                    PsVerifierContextValue::MatchGroup
                )
                .clone(),
            );
        }

        let result = OriginResponseStage {
            match_groups: verified_mgs,
            aug: PsVerificationKey::None,
        };

        Ok(PsVerifierContext {
            scopes: c.scopes.clone(),
            value: make_context_value!(
                result,
                PsVerifierContextValue::OriginResponseStage,
                PsVerificationKey,
                TypedOriginResponseStage
            ),
        })
    }
}

pub(crate) fn verifier() -> (PsVerifier, PsVerifierContext) {
    (PsVerifier {}, PsVerifierContext::default())
}

/// Verify the (semantic) validity of some CDNI Processing Stages JSON
///
/// In particular, test whether
/// 1. the MEL expressions have the proper types, and
/// 2. the generic metadata type names are correct.
///
pub fn verify_ps(
    stages: &TypedProcessingStages<()>,
) -> Result<TypedProcessingStages<PsVerificationKey>, Box<PsVerificationError>> {
    let (mut verifier, context) = verifier();
    let value = &stages;
    let result = verifier.visit_processing_stages(value, &context)?;
    Ok(expect_some_value!(&result.value, PsVerifierContextValue::ProcessingStages).clone())
}

/// Verify the (semantic) validity of some CDNI Processing Stages Request Stage JSON
///
pub fn verify_ps_request_stage(
    stage: &TypedGenericStage,
    scopes: Scopes<Type>,
) -> Result<TypedStage<PsVerificationKey>, Box<PsVerificationError>> {
    match stage.typed()? {
        TypedStage::ClientRequest(crq) => {
            let (mut verifier, mut context) = verifier();
            context.scopes = scopes;
            let result = verifier.visit_client_request_stage(&crq, &context)?;
            Ok(TypedStage::ClientRequest(
                expect_some_value!(&result.value, PsVerifierContextValue::ClientRequestStage)
                    .clone(),
            ))
        }
        TypedStage::ClientResponse(crs) => {
            let (mut verifier, mut context) = verifier();
            context.scopes = scopes;
            let result = verifier.visit_client_response_stage(&crs, &context)?;
            Ok(TypedStage::ClientResponse(
                expect_some_value!(&result.value, PsVerifierContextValue::ClientResponseStage)
                    .clone(),
            ))
        }
        TypedStage::OriginRequest(orq) => {
            let (mut verifier, mut context) = verifier();
            context.scopes = scopes;
            let result = verifier.visit_origin_request_stage(&orq, &context)?;
            Ok(TypedStage::OriginRequest(
                expect_some_value!(&result.value, PsVerifierContextValue::OriginRequestStage)
                    .clone(),
            ))
        }
        TypedStage::OriginResponse(ors) => {
            let (mut verifier, mut context) = verifier();
            context.scopes = scopes;
            let result = verifier.visit_origin_response_stage(&ors, &context)?;
            Ok(TypedStage::OriginResponse(
                expect_some_value!(&result.value, PsVerifierContextValue::OriginResponseStage)
                    .clone(),
            ))
        }
    }
}

#[cfg(test)]
mod test_verify {
    use std::assert_matches;

    use crate::mel::tvs::Type;
    use crate::ps::spec::{
        HeaderTransform, ResponseTransform, SyntheticResponse, TypedHeaderTransform,
        TypedSyntheticResponse,
    };
    use crate::ps::verify::{PsVerifierContextValue, verifier};
    use crate::ps::visit::PsVisitor;
    use crate::{
        mel::ast::Expr::BinaryExpr,
        ps::{
            spec::TypedResponseTransform,
            verify::{
                PsVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                PsVerificationKey, verify_ps,
            },
        },
    };

    use crate::ps::tests::test_helpers::{
        expression_match, generic_metadata, header_transform, processing_stages, request_transform,
        response_transform, synthetic_response, typed_header,
    };

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let mut stages = processing_stages(vec![], vec![], vec![], vec![]);
        stages.tpe = "MI.ProcessigStages".to_string();

        let result = verify_ps(&stages)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(*result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_generic_metadata_with_bad_type_name() {
        let generic = generic_metadata("Mi.CachePolicy", None);

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_generic_metadata(&generic, &context)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(*result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let mtch = expression_match("5 + 4");
        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_expression_match(&mtch, &context)
            .expect_err("Could verify PS expression match with incorrect MEL type");

        assert_matches!(*result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let xform = request_transform(None, Some("5+4".to_string()), Some(true));
        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_request_transform(&xform, &context)
            .expect_err("Could verify PS with incorrect MEL type of expression to calculate URI");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_response_transform() {
        let xform = response_transform(None, Some("404".to_string()), None, None);
        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            result,
            PsVerifierContextValue::ResponseTransform(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr() {
        let xform = response_transform(None, Some("400+4".to_string()), Some(true), None);
        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            result,
            PsVerifierContextValue::ResponseTransform(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: Some(true),
                    synthetic: None,
                    aug: PsVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }
    #[test]
    fn test_verify_response_transform_rs_expr_wrong_type() {
        let xform = response_transform(None, Some("false".to_string()), Some(true), None);
        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect_err("Could verify PS response transform with incorrect MEL type of expression to calculate response status");

        assert_matches!(*result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let header_to_delete = vec!["delete1".to_string(), "delete2".to_string()];

        let header_xform = header_transform(
            Some(header_to_delete),
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::HeaderTransform(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: Some(d),
                            add: Some(a),
                            replace: Some(b),
                            aug: PsVerificationKey::None
                        }
                    }) if d.len() == 2 && a.len() == 1 && b.len() == 1
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr() {
        let header_to_add = typed_header("add", "\"value\"", Some(true));
        let header_to_replace = typed_header("replace", "\"testing\" . \"one\"", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::HeaderTransform(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: None,
                            add: Some(a),
                            replace: Some(b),
                            aug: PsVerificationKey::None
                        }
            }) if matches!(a[0].value.aug, PsVerificationKey::Expr(_)) && matches!(b[0].value.aug, PsVerificationKey::Expr(_))
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_add() {
        let header_to_add = typed_header("add", "5", Some(true));
        let header_to_replace = typed_header("replace", "\"testing\"", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect_err("Could verify PS typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_replace() {
        let header_to_add = typed_header("add", "\"value\"", Some(true));
        let header_to_replace = typed_header("replace", "true", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect_err("Could verify PS typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, None, None, None, None);

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: PsVerificationKey::ExprPair(None, None)
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_response_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, Some("400 + 4".to_string()), Some(true), None, None);

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: PsVerificationKey::ExprPair(Some(_), None)
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_body_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(
            headers,
            None,
            None,
            Some("\"testing\"".to_string()),
            Some(true),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: PsVerificationKey::ExprPair(None, Some(_))
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_body_and_response_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(
            headers,
            Some("400 + 4".to_string()),
            Some(true),
            Some("\"testing\"".to_string()),
            Some(true),
        );

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct PS")
            .value
            .expect("No value in PS Verification Context");

        assert_matches!(
            &result,
            PsVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: PsVerificationKey::ExprPair(Some(_), Some(_))
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_wrong_response_expr_type() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, Some("true".to_string()), Some(true), None, None);

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect_err("Could verify PS synthetic response with incorrect MEL type of expression to calculate response");

        assert_matches!(*result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response_wrong_body_expr_type() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, None, None, Some("true".to_string()), Some(true));

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect_err("Could verify PS synthetic response with incorrect MEL type of expression to calculate body");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Boolean));
    }
}

#[cfg(test)]
mod test_verify_from_json {
    use std::assert_matches;

    use crate::mel::scope::Scopes;
    use crate::mel::tvs::Type;
    use crate::ps::spec::{
        HeaderTransform, ResponseTransform, SyntheticResponse, TypedGenericStage,
        TypedHeaderTransform, TypedStage, TypedSyntheticResponse,
    };
    use crate::ps::verify::verify_ps_request_stage;
    use crate::tests::read_test_file;
    use crate::{
        mel::ast::Expr::BinaryExpr,
        ps::{
            spec::{
                RequestTransform, TypedProcessingStages, TypedRequestTransform,
                TypedResponseTransform,
            },
            verify::{
                PsVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                PsVerificationKey, verify_ps,
            },
        },
    };

    use std::path::Path;

    #[test]
    fn test_verify_simple() {
        let json = read_test_file(Path::new("./src/ps/tests/simple/deserialize_verify.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify proper PS");

        let result_serialized =
            serde_json::to_string_pretty(&result).expect("Could not serialize verified PS");
        pretty_assertions::assert_eq!(json, result_serialized);
    }

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/generic_metadata/bad-typename.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(*result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_stage_metadata_with_generic_metadata_bad_type_name() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/generic_metadata/bad-typename2.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect_err("Could verify mistyped PS");

        assert_matches!(*result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/stage_rules/wrong_match_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS expression match with incorrect MEL type");

        assert_matches!(*result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/request_transform/uri_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .request_xform,
            Some(TypedRequestTransform {
                tpe: _,
                value: RequestTransform {
                    xform: _,
                    uri: _,
                    uri_is_expr: Some(false),
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_request_transform_uri_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/request_transform/uri_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .request_xform,
            Some(TypedRequestTransform {
                tpe: _,
                value: RequestTransform {
                    xform: _,
                    uri: _,
                    uri_is_expr: Some(true),
                    aug: PsVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/request_transform/uri_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS with incorrect MEL type of expression to calculate URI");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_response_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/response_transform/rs_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/response_transform/rs_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: Some(true),
                    synthetic: None,
                    aug: PsVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/response_transform/rs_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS response transform with incorrect MEL type of expression to calculate response status");

        assert_matches!(*result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: Some(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: Some(d),
                            add: Some(a),
                            replace: Some(b),
                            aug: PsVerificationKey::None
                        }
                    }),
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: PsVerificationKey::None
                }
            }) if d.len() == 2 && a.len() == 1 && b.len() == 1
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/header_transform/value_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: Some(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: _,
                            add: Some(a),
                            replace: Some(b),
                            aug: PsVerificationKey::None
                        }
                    }),
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: PsVerificationKey::None
                }
            }) if matches!(a[0].value.aug, PsVerificationKey::Expr(_)) && matches!(b[0].value.aug, PsVerificationKey::Expr(_))
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_add() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_replace() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_expr_wrong_type_replace.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result =
            verify_ps(&stages).expect_err("Could verify PS with invalid metadata type name");
        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response() {
        let json = read_test_file(Path::new("./src/ps/tests/synthetic_response/no_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: PsVerificationKey::ExprPair(None, None)
                        }
                    }),
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_response_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/response_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: PsVerificationKey::ExprPair(Some(_), None)
                        }
                    }),
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_body_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/body_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: PsVerificationKey::ExprPair(None, Some(_))
                        }
                    }),
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_body_and_response_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/both_body_response_exprs.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages).expect("Could not verify correct PS");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: PsVerificationKey::ExprPair(Some(_), Some(_))
                        }
                    }),
                    aug: PsVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_wrong_response_expr_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/response_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS synthetic response with incorrect MEL type of expression to calculate response");

        assert_matches!(*result, ExpressionWrongType(Type::Integer, Type::String));
    }

    #[test]
    fn test_verify_synthetic_response_wrong_body_expr_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/body_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps(&stages)
            .expect_err("Could verify PS synthetic response with incorrect MEL type of expression to calculate body");

        assert_matches!(*result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_client_request_stage() {
        let json = read_test_file(Path::new("./src/ps/tests/client_request_stage/if.json"));
        let stages = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps_request_stage(&stages, Scopes::<Type>::default())
            .expect("Could not verify PS client request stage");

        assert_matches!(result, TypedStage::ClientRequest(_))
    }

    #[test]
    fn test_verify_client_request_stage_request_response_header_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/client_request_stage/request_response_header_transform.json",
        ));
        let stages = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_ps_request_stage(&stages, Scopes::<Type>::default())
            .expect("Could not verify PS client request stage");

        assert_matches!(result, TypedStage::ClientRequest(_))
    }
}
