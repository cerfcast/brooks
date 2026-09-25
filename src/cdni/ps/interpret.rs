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

use http::{HeaderName, HeaderValue, StatusCode};

use crate::{
    cdni::{
        gmd::spec::TypedGenericMetadata,
        mi::{MetadataInformationResultElement, MetadataInformationResultElements},
        ps::{
            interpret::{
                PsInterpretMode::HeaderCalculate,
                PsInterpretValue::{Header, MatchNo, MatchYes},
                PsInterpretValueType::{MatchResult, SyntheticResponse, Terminate},
            },
            spec::{
                TypedClientRequestStage, TypedExpressionMatch, TypedHeader, TypedHeaderTransform,
                TypedMatchGroup, TypedProcessingStages, TypedRequestTransform,
                TypedResponseTransform, TypedStage, TypedStageMetadata, TypedStageRules,
                TypedSyntheticResponse,
            },
            verify::PsVerificationKey,
            visit::{PsVisitor, PsVisitorResult},
        },
    },
    environment::scope::Scopes,
    logging::LogMsgs,
    mel::{
        self,
        analysis::Analyzed,
        ast::Expr,
        interpreter::interpret::{
            MelInterpAssertion, MelInterpContext, MelInterpError, MelInterpLocatableError,
            TypedValue, Value,
        },
        types::Type,
    },
    tools::prr,
};

use std::{
    error::Error,
    fmt::{Debug, Display},
    str::FromStr,
};

pub type PsInterpretResult =
    Result<(PsInterpretValue, MetadataInformationResultElements), Box<PsInterpretError>>;

#[derive(Debug, Clone)]
pub enum PsInterpretAssertionFailures {
    MissingAnalyzedExpression,
    MissingInterpreterExpressionValue,
    InvalidInterpreterMode,
    MissingResult,
}

impl Display for PsInterpretAssertionFailures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsInterpretAssertionFailures::MissingAnalyzedExpression => {
                write!(f, "Missing information about an analyzed expression")
            }
            PsInterpretAssertionFailures::MissingInterpreterExpressionValue => {
                write!(f, "Missing value from an interpreted expression")
            }
            PsInterpretAssertionFailures::InvalidInterpreterMode => {
                write!(f, "Invalid interpreter mode")
            }
            PsInterpretAssertionFailures::MissingResult => write!(f, "Missing result"),
        }
    }
}

#[derive(Debug, Default)]
pub enum PsInterpretError {
    #[default]
    NoError,
    AssertionFailure(PsInterpretAssertionFailures),
    MelInterpreterError(Box<MelInterpLocatableError>),
    InvalidRequest,
    InvalidUri(url::ParseError),
    InvalidResponse(String),
    WrongType(PsInterpretValueType, PsInterpretValueType),
    WrongMatchGroupValueType(PsInterpretValueType),
    ProcessableRequestResponseError(prr::Error),
}

impl Error for PsInterpretError {}

impl Display for PsInterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsInterpretError::NoError => write!(f, "No Error"),
            PsInterpretError::AssertionFailure(af) => {
                write!(f, "Assertion failure: {af}")
            }
            PsInterpretError::MelInterpreterError(meli) => {
                write!(f, "MEL Interpreter error: {}", meli)
            }
            PsInterpretError::InvalidRequest => write!(f, "Invalid HTTP request"),
            PsInterpretError::InvalidUri(iuri) => write!(f, "Invalid URI: {iuri}"),
            PsInterpretError::InvalidResponse(response) => {
                write!(f, "Invalid Response: {response}")
            }
            PsInterpretError::WrongType(expected, actual) => {
                write!(f, "Wrong type: expected: {expected} actual: {actual}",)
            }
            PsInterpretError::WrongMatchGroupValueType(actual) => {
                write!(f, "Match group value should not have {actual} type")
            }
            PsInterpretError::ProcessableRequestResponseError(
                processable_request_response_error,
            ) => {
                write!(
                    f,
                    "Error occurred when modifying the request/response: {processable_request_response_error}"
                )
            }
        }
    }
}

// CDNI Processing Stage Interpreter

pub(crate) struct PsInterpreter<'a> {
    pub rr: &'a mut dyn prr::Prr<Vec<u8>>,
}

impl<'a> PsInterpreter<'a> {
    pub fn install_generic_visitors(&mut self) {}

    fn evaluate_mel_expr(
        &self,
        mel_scopes: Scopes<TypedValue>,
        expr: &Expr<Analyzed>,
        expected: Type,
    ) -> Result<TypedValue, PsInterpretError> {
        let expr_context = MelInterpContext {
            val: None,
            scopes: mel_scopes,
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
        };

        let result_ctxt = mel::interpreter::interpret(expr, expr_context)
            .map_err(PsInterpretError::MelInterpreterError)?;

        let result_val = result_ctxt
            .val
            .as_ref()
            .ok_or(PsInterpretError::AssertionFailure(
                PsInterpretAssertionFailures::MissingInterpreterExpressionValue,
            ))?;

        if result_val.tpe == expected {
            Ok(result_val.clone())
        } else {
            Err(PsInterpretError::MelInterpreterError(
                MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(expected, result_val.tpe.clone()).into(),
                    )
                    .into(),
                    context: result_ctxt,
                    location: expr.location().clone(),
                }
                .into(),
            ))
        }
    }

    fn interpret_match_groups_in_stage(
        &mut self,
        mgs: &Vec<TypedMatchGroup<PsVerificationKey>>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        for mg in mgs {
            c = self.visit_match_group(mg, c)?;
            match c.result {
                Some(PsInterpretValue::Terminate | PsInterpretValue::SyntheticResponse(_)) => {
                    // This result stops processing.
                    break;
                }
                Some(PsInterpretValue::MatchYes | PsInterpretValue::MatchNo) => {
                    // This result continues processing.
                    continue;
                }
                Some(r) => {
                    return Err(PsInterpretError::WrongType(
                        PsInterpretValueType::Terminate,
                        r.into(),
                    ));
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            }
        }
        if let Some(result) = c.result {
            Ok(PsInterpretContext {
                mode: c.mode,
                result: Some(result.clone()),
                metadata: c.metadata,
                mel_scopes: c.mel_scopes,
            })
        } else {
            Err(PsInterpretError::AssertionFailure(
                PsInterpretAssertionFailures::MissingResult,
            ))
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum PsInterpretMode {
    Request,
    Response,
    HeaderAdd,
    HeaderReplace,
    HeaderCalculate,
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub enum PsInterpretValueType {
    SyntheticResponse,
    Terminate,
    MatchResult,
    Header,
}

impl Display for PsInterpretValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SyntheticResponse => write!(f, "Synthetic response"),
            Terminate => write!(f, "Terminate"),
            MatchResult => write!(f, "Match result"),
            PsInterpretValueType::Header => write!(f, "Header"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PsInterpretValue {
    SyntheticResponse(http::Response<Vec<u8>>),
    Terminate,
    MatchYes,
    MatchNo,
    Header(String, String),
}

impl From<PsInterpretValue> for PsInterpretValueType {
    fn from(value: PsInterpretValue) -> Self {
        match value {
            PsInterpretValue::SyntheticResponse(_) => SyntheticResponse,
            PsInterpretValue::Terminate => Terminate,
            MatchYes | MatchNo => MatchResult,
            PsInterpretValue::Header(_, _) => PsInterpretValueType::Header,
        }
    }
}

impl From<bool> for PsInterpretValue {
    fn from(value: bool) -> Self {
        if value { MatchYes } else { MatchNo }
    }
}

#[derive(Debug, Default)]
pub(crate) struct PsInterpretContext {
    pub mode: PsInterpretMode,
    pub result: Option<PsInterpretValue>,
    pub metadata: MetadataInformationResultElements,
    pub mel_scopes: Scopes<TypedValue>,
}

impl PsInterpretContext {
    fn update_mode(self, new_mode: PsInterpretMode) -> PsInterpretContext {
        let mut nc = self;
        nc.mode = new_mode;
        nc
    }
    fn update_result(self, new_result: Option<PsInterpretValue>) -> PsInterpretContext {
        let mut nc = self;
        nc.result = new_result;
        nc
    }

    fn update_metadata_elements(
        self,
        new_element: Box<dyn MetadataInformationResultElement>,
    ) -> PsInterpretContext {
        let mut nc = self;
        nc.metadata.elements.push(new_element);
        nc
    }
    fn replace_metadata_elements(
        self,
        new_elements: MetadataInformationResultElements,
    ) -> PsInterpretContext {
        let mut nc = self;
        nc.metadata = new_elements;
        nc
    }
}

impl<'a> PsVisitor<PsVerificationKey, PsInterpretContext, PsInterpretContext, PsInterpretError>
    for PsInterpreter<'a>
{
    fn visit_processing_stages(
        &mut self,
        _v: &TypedProcessingStages<PsVerificationKey>,
        _c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_rules(
        &mut self,
        v: &TypedStageRules<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let doq = if let Some(mtch) = &v.value.mtch {
            let expr = match &mtch.value.aug {
                PsVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };
            let result = self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::Boolean)?;
            match result {
                TypedValue {
                    tpe: Type::Boolean,
                    value: Value::Boolean(v),
                } => v,
                _ => unreachable!(),
            }
        } else {
            true
        };

        if doq {
            self.visit_stage_metadata(&v.value.stage_metadata, c.update_result(Some(doq.into())))
        } else {
            Ok(c.update_result(Some(doq.into())))
        }
    }

    fn visit_expression_match(
        &mut self,
        _v: &TypedExpressionMatch<PsVerificationKey>,
        _c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_metadata(
        &mut self,
        v: &TypedStageMetadata<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(generic) = &v.value.generic {
            for generic in generic {
                c = self.visit_generic_metadata(generic, c)?;
            }
        }

        match &c.mode {
            PsInterpretMode::Request => {
                if let Some(req_xform) = &v.value.request_xform {
                    return self.visit_request_transform(req_xform, c);
                }
            }
            PsInterpretMode::Response => {
                if let Some(res_xform) = &v.value.response_xform {
                    return self.visit_response_transform(res_xform, c);
                }
            }
            _ => {
                return Err(PsInterpretError::AssertionFailure(
                    PsInterpretAssertionFailures::InvalidInterpreterMode,
                ));
            }
        }
        Ok(c)
    }

    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(header_xform) = &v.value.xform {
            c = self.visit_header_transform(
                header_xform,
                PsInterpretContext {
                    mode: PsInterpretMode::Request,
                    metadata: c.metadata,
                    result: c.result.clone(),
                    mel_scopes: c.mel_scopes,
                },
            )?;
        }

        if let Some(new_uri) = &v.value.uri {
            let new_uri = if let Some(uri_is_expr) = &v.value.uri_is_expr
                && *uri_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::Expr(expr) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::String)?;
                match result {
                    TypedValue {
                        tpe: Type::String,
                        value: Value::String(s),
                    } => url::Url::from_str(&s),
                    _ => unreachable!(),
                }
            } else {
                url::Url::from_str(&new_uri.clone())
            }
            .map_err(PsInterpretError::InvalidUri)?;

            self.rr
                .set_url(&new_uri)
                .map_err(PsInterpretError::ProcessableRequestResponseError)?
        }

        Ok(c)
    }

    fn visit_response_transform(
        &mut self,
        v: &TypedResponseTransform<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(synthetic_response) = &v.value.synthetic {
            return self.visit_synthetic_response(synthetic_response, c);
        }

        if let Some(header_xform) = &v.value.xform {
            c = self
                .visit_header_transform(header_xform, c.update_mode(PsInterpretMode::Response))?;
        }

        if let Some(new_response) = &v.value.response_status {
            let new_response = if let Some(response_is_expr) = &v.value.response_status_expr
                && *response_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::Expr(expr) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tpe: Type::Integer,
                        value: Value::Integer(i),
                    } => Ok(i),
                    _ => unreachable!(),
                }
            } else {
                new_response.parse::<i64>()
            }
            .map_err(|e| PsInterpretError::InvalidResponse(e.to_string()))?;
            let new_response = u16::try_from(new_response).map_err(|e| {
                PsInterpretError::InvalidResponse(format!(
                    "Could not convert {new_response} to unsigned 16-bit number: {e}"
                ))
            })?;
            self.rr.set_status(&new_response).map_err(|_| {
                PsInterpretError::ProcessableRequestResponseError(prr::Error::BadValue(
                    prr::Value {
                        tpe: prr::ValueType::Status,
                        value: Some(new_response.to_string()),
                    },
                ))
            })?;
        }

        Ok(c)
    }

    fn visit_generic_metadata(
        &mut self,
        _v: &TypedGenericMetadata<PsVerificationKey>,
        _c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(to_delete) = &v.value.delete {
            for htr in to_delete {
                self.rr
                    .remove_header(htr)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?
            }
        }

        if let Some(to_add) = &v.value.add {
            for htr in to_add {
                c = self.visit_header(htr, c.update_mode(PsInterpretMode::HeaderAdd))?;
            }
        }

        if let Some(to_replace) = &v.value.replace {
            for htr in to_replace {
                c = self.visit_header(htr, c.update_mode(PsInterpretMode::HeaderReplace))?;
            }
        }

        Ok(c)
    }

    fn visit_header(
        &mut self,
        v: &TypedHeader<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let value = if let Some(expr) = &v.value.value_expr
            && *expr
        {
            let expr = match &v.value.aug {
                PsVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };
            let result = self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::String)?;
            match result {
                TypedValue {
                    tpe: Type::String,
                    value: Value::String(s),
                } => s,
                _ => unreachable!(),
            }
        } else {
            v.value.value.clone()
        };

        match c.mode {
            PsInterpretMode::HeaderAdd => {
                self.rr
                    .add_header(&v.value.name, &value)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?;
                Ok(c)
            }
            PsInterpretMode::HeaderReplace => {
                self.rr
                    .set_header_value(&v.value.name, &value)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?;
                Ok(c)
            }
            PsInterpretMode::HeaderCalculate => {
                Ok(c.update_result(Some(Header(v.value.name.clone(), value))))
            }
            _ => todo!(),
        }
    }

    fn visit_synthetic_response(
        &mut self,
        v: &TypedSyntheticResponse<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut response = http::Response::builder();

        if let Some(headers) = &v.value.headers {
            for header in headers {
                //let result = self.visit_header(header, c.update_mode(HeaderCalculate))?;

                c = self.visit_header(header, c.update_mode(HeaderCalculate))?;

                match &c.result {
                    Some(PsInterpretValue::Header(name, value)) => {
                        response.headers_mut().unwrap().insert(
                            HeaderName::from_bytes(name.as_bytes()).expect("Todo"),
                            HeaderValue::from_str(value).expect("Todo"),
                        );
                    }
                    Some(r) => {
                        return Err(PsInterpretError::WrongType(
                            PsInterpretValueType::Header,
                            r.clone().into(),
                        ));
                    }
                    None => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                }
            }
        }

        response = response.status(if let Some(new_response) = &v.value.response_status {
            if let Some(response_is_expr) = &v.value.response_status_expr
                && *response_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::ExprPair(Some(expr), _) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tpe: Type::Integer,
                        value: Value::Integer(i),
                    } => StatusCode::from_u16(i as u16),
                    _ => unreachable!(),
                }
            } else {
                new_response.parse::<StatusCode>()
            }
            .map_err(|e| PsInterpretError::InvalidResponse(e.to_string()))?
        } else {
            StatusCode::OK
        });

        let response = response
            .body(if let Some(body) = &v.value.body {
                if let Some(body_is_expr) = &v.value.body_expr
                    && *body_is_expr
                {
                    let expr = match &v.value.aug {
                        PsVerificationKey::ExprPair(_, Some(expr)) => expr,
                        _ => {
                            return Err(PsInterpretError::AssertionFailure(
                                PsInterpretAssertionFailures::MissingAnalyzedExpression,
                            ));
                        }
                    };
                    let result =
                        self.evaluate_mel_expr(c.mel_scopes.clone(), expr, Type::String)?;
                    match result {
                        TypedValue {
                            tpe: Type::String,
                            value: Value::String(s),
                        } => s.into_bytes(),
                        _ => unreachable!(),
                    }
                } else {
                    body.to_owned().into_bytes()
                }
            } else {
                "".to_string().into_bytes()
            })
            .expect("Could not create HTTP response");

        Ok(c.update_result(Some(PsInterpretValue::SyntheticResponse(response))))
    }

    fn visit_match_group(
        &mut self,
        v: &TypedMatchGroup<PsVerificationKey>,
        mut c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let else_ifs = v
            .value
            .else_ifs
            .as_ref()
            .map(|else_ifs| else_ifs.iter())
            .unwrap_or([].iter());

        let rules = [&v.value.if_rule].into_iter().chain(else_ifs);
        for r in rules {
            c = self.visit_stage_rules(r, c)?;
            match &c.result {
                Some(PsInterpretValue::MatchNo) => {
                    continue; // do the next rule.
                }
                Some(_) => {
                    break; // do _not_ do the next rule in any other case.
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            };
        }
        if let Some(result) = &c.result {
            Ok(PsInterpretContext {
                mode: c.mode,
                result: Some(result.clone()),
                metadata: c.metadata,
                mel_scopes: c.mel_scopes,
            })
        } else {
            Err(PsInterpretError::AssertionFailure(
                PsInterpretAssertionFailures::MissingResult,
            ))
        }
    }

    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_origin_request_stage(
        &mut self,
        v: &super::spec::TypedOriginRequestStage<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_client_response_stage(
        &mut self,
        v: &super::spec::TypedClientResponseStage<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_origin_response_stage(
        &mut self,
        v: &super::spec::TypedOriginResponseStage<PsVerificationKey>,
        c: PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }
}

/// TODO: Document
pub fn interpret_stage(
    ts: &TypedStage<PsVerificationKey>,
    mel_scopes: &Scopes<TypedValue>,
    reqres: &mut dyn prr::Prr<Vec<u8>>,
    mode: PsInterpretMode,
) -> PsInterpretResult {
    let mut visitor = PsInterpreter { rr: reqres };

    visitor.install_generic_visitors();

    let context = PsInterpretContext {
        mel_scopes: mel_scopes.clone(),
        mode,
        ..Default::default()
    };

    let result = match ts {
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
    }?;

    let md = result.metadata;
    let result = result
        .result
        .ok_or(Box::new(PsInterpretError::AssertionFailure(
            PsInterpretAssertionFailures::MissingResult,
        )))?;
    Ok((result, md))
}

#[derive(Debug)]
enum EffectfulRequestActions {
    DeleteHeader(String),
    AddHeader(String, String),
    ClearHeaders,
    SetUri(String),
    SetBody,
    SetResponse(u16),
}

#[derive(Debug)]
pub(crate) struct EffectfulProcessableRequestResponse {
    log: Vec<EffectfulRequestActions>,
    tpe: prr::PrrType,
}

impl EffectfulProcessableRequestResponse {
    fn new_request() -> Self {
        EffectfulProcessableRequestResponse {
            log: Default::default(),
            tpe: prr::PrrType::Request,
        }
    }

    fn new_response() -> Self {
        EffectfulProcessableRequestResponse {
            log: Default::default(),
            tpe: prr::PrrType::Response,
        }
    }
}

impl prr::Prr<Vec<u8>> for EffectfulProcessableRequestResponse {
    fn header_value(&self) -> Option<HeaderValue> {
        None
    }

    fn headers(&self) -> Vec<(String, HeaderValue)> {
        vec![]
    }

    fn set_header_value(&mut self, _header: &str, _value: &str) -> Result<(), prr::Error> {
        Ok(())
    }

    fn clear_headers(&mut self) -> prr::Result<()> {
        self.log.push(EffectfulRequestActions::ClearHeaders);
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> Result<(), prr::Error> {
        self.log
            .push(EffectfulRequestActions::DeleteHeader(header.to_string()));
        Ok(())
    }

    fn url(&self) -> Result<url::Url, prr::Error> {
        Ok(url::Url::from_str("http://www.example.com/").expect("Could not make default URL"))
    }

    fn set_url(&mut self, url: &url::Url) -> Result<(), prr::Error> {
        self.log
            .push(EffectfulRequestActions::SetUri(url.to_string()));
        Ok(())
    }

    fn set_status(&mut self, response: &u16) -> Result<(), prr::Error> {
        self.log
            .push(EffectfulRequestActions::SetResponse(*response));
        Ok(())
    }

    fn add_header(&mut self, header: &str, value: &str) -> Result<(), prr::Error> {
        self.log.push(EffectfulRequestActions::AddHeader(
            header.to_string(),
            value.to_string(),
        ));
        Ok(())
    }

    fn set_body(&mut self, _: &Vec<u8>) -> prr::Result<()> {
        self.log.push(EffectfulRequestActions::SetBody);
        Ok(())
    }

    fn get_body(&self) -> prr::Result<&Vec<u8>> {
        todo!()
    }

    fn get_status(&self) -> prr::Result<StatusCode> {
        todo!()
    }

    fn tpe(&self) -> prr::PrrType {
        self.tpe.clone()
    }

    fn set_method(&mut self, _method: &http::Method) -> prr::Result<()> {
        todo!()
    }

    fn get_method(&self) -> http::Method {
        todo!()
    }
}

#[cfg(test)]
mod ps_interpreter_tests {
    use crate::{
        cdni::ps::{
            interpret::{
                EffectfulProcessableRequestResponse, EffectfulRequestActions, PsInterpretMode,
                PsInterpretValue, interpret_stage,
            },
            spec::{TypedGenericStage, TypedStage},
            verify::{PsVerifierContextValue, verifier, verify_ps_request_stage},
            visit::PsVisitor,
        },
        cdni::tests::test_helpers::{
            client_request_stage, expression_match, match_group, request_transform,
            response_transform, stage_metadata, synthetic_response, typed_header, typed_stage_rule,
        },
        environment::scope::Scopes,
        mel::types::Type,
        tests::read_test_file,
    };
    use std::assert_matches;
    use std::path::Path;

    #[test]
    fn test_interpret_client_request_stage() {
        let json = read_test_file(Path::new(
            "./src/cdni/ps/tests/client_request_stage/if.json",
        ));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, &Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");

        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &result,
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_response_header_transform() {
        let json = read_test_file(Path::new(
            "./src/cdni/ps/tests/client_request_stage/request_response_header_transform.json",
        ));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, &Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &result,
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_uri_transform() {
        let mut request_xform_stage = stage_metadata();

        let request_xform_match = expression_match("true");
        let request_xform = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_stage.value.request_xform = Some(request_xform);

        let request_xform_mg = match_group(
            typed_stage_rule(Some(request_xform_match), request_xform_stage),
            None,
        );

        let crs = client_request_stage(vec![request_xform_mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_request_uri_transform_if() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("true");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );
        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![typed_stage_rule(
                Some(request_xform_true_match),
                request_xform_true_stage,
            )]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_uri_transform_else() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_false_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("false");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_false_false_match = expression_match("true");
        let request_xform_false_false = request_transform(
            None,
            Some("\"http://\" . \"example1.com\"".to_string()),
            Some(true),
        );
        request_xform_false_false_stage.value.request_xform = Some(request_xform_false_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );

        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(request_xform_false_false_match),
                    request_xform_false_false_stage,
                ),
                typed_stage_rule(Some(request_xform_true_match), request_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example1.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_request_uri_transform_else_if() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_false_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("false");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_false_false_match = expression_match("false");
        let request_xform_false_false = request_transform(
            None,
            Some("\"http://\" . \"example1.com\"".to_string()),
            Some(true),
        );
        request_xform_false_false_stage.value.request_xform = Some(request_xform_false_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );

        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(request_xform_false_false_match),
                    request_xform_false_false_stage,
                ),
                typed_stage_rule(Some(request_xform_true_match), request_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example2.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_status_transform() {
        let mut response_xform_stage = stage_metadata();

        let response_xform_match = expression_match("true");
        let response_xform = response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_stage.value.response_xform = Some(response_xform);

        let response_xform_mg = match_group(
            typed_stage_rule(Some(response_xform_match), response_xform_stage),
            None,
        );

        let crs = client_request_stage(vec![response_xform_mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(9));
    }

    fn test_interpret_client_request_stage_response_status_transform_if() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("true");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![typed_stage_rule(
                Some(response_xform_true_match),
                response_xform_true_stage,
            )]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(9));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_status_transform_else() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_false_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("false");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_false_false_match = expression_match("true");
        let response_xform_false_false =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_false_false_stage.value.response_xform = Some(response_xform_false_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 6".to_string()), Some(true), None);

        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(response_xform_false_false_match),
                    response_xform_false_false_stage,
                ),
                typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(10));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_response_status_transform_else_if() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_false_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("false");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_false_false_match = expression_match("false");
        let response_xform_false_false =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_false_false_stage.value.response_xform = Some(response_xform_false_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 6".to_string()), Some(true), None);

        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(response_xform_false_false_match),
                    response_xform_false_false_stage,
                ),
                typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(11));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_synthetic() {
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_true_match = expression_match("true");
        let response_xform_true = response_transform(
            None,
            None,
            None,
            Some(synthetic_response(
                Some(vec![
                    typed_header("X-custom1", "Custom value 1", None),
                    typed_header("X-custom2", "Custom value 2", None),
                ]),
                Some("400 + 4".to_string()),
                Some(true),
                Some("\"This \" . \"is \" . \"a \" . \"test.\"".to_string()),
                Some(true),
            )),
        );
        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            None,
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::new_request();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &Scopes { scopes: vec![] },
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 0);
        assert_matches!(&result.0, PsInterpretValue::SyntheticResponse(r)
            if r.status() == 404 && r.body() == &"This is a test.".to_string().into_bytes() && r.headers().len() == 2);
    }
}
