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

use http::{HeaderName, HeaderValue, StatusCode, Uri, uri::InvalidUri};

use crate::{
    logging::LogMsgs,
    mel::{
        analysis::Analyzed,
        ast::Expr,
        interpreter::{
            self,
            interpret::{
                MelInterpContext, MelInterpError, MelInterpLocatableError, TypedValue, Value,
            },
        },
        scope::{Scope, Scopes},
        tvs::Type,
    },
    ps::{
        interpret::{
            PsInterpretError::{WrongMatchGroupValueType, WrongType},
            PsInterpretMode::HeaderCalculate,
            PsInterpretValue::{Header, MatchNo, MatchYes},
            PsInterpretValueType::{MatchResult, SyntheticResponse, Terminate},
        },
        spec::{
            TypedClientRequestStage, TypedExpressionMatch, TypedGenericMetadata, TypedHeader,
            TypedHeaderTransform, TypedMatchGroup, TypedProcessingStages, TypedRequestTransform,
            TypedResponseTransform, TypedStage, TypedStageMetadata, TypedStageRules,
            TypedSyntheticResponse,
        },
        verify::PsVerificationKey,
        visit::{PsVisitor, PsVisitorResult},
    },
};

use std::fmt::Debug;

/// A request that can be manipulated by interpretation processing stages.
pub trait ProcessableRequestResponse: Debug {
    fn header_value(&self) -> &str;
    fn headers(&self) -> &[&str];
    fn set_header_value(&mut self, header: &str, value: &str);
    fn remove_header(&mut self, header: &str);
    fn add_header(&mut self, header: &str, value: &str);
    fn uri(&self) -> Uri;
    fn set_uri(&mut self, uri: &Uri);
    fn set_response(&mut self, response: &i64);
}

pub type PsInterpretResult = Result<PsInterpretValue, PsInterpretError>;

#[derive(Debug, Clone)]
pub enum PsInterpretAssertionFailures {
    MissingAnalyzedExpression,
    MissingInterpreterExpressionValue,
    InvalidInterpreterMode,
    MissingResult,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for PsInterpretAssertionFailures {
    fn to_string(&self) -> String {
        match self {
            PsInterpretAssertionFailures::MissingAnalyzedExpression => {
                "Missing information about an analyzed expression".to_string()
            }
            PsInterpretAssertionFailures::MissingInterpreterExpressionValue => {
                "Missing value from an interpreted expression".to_string()
            }
            PsInterpretAssertionFailures::InvalidInterpreterMode => {
                "Invalid interpreter mode".to_string()
            }
            PsInterpretAssertionFailures::MissingResult => "Missing result".to_string(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Default)]
pub enum PsInterpretError {
    #[default]
    NoError,
    AssertionFailure(PsInterpretAssertionFailures),
    MelInterpreterError(MelInterpLocatableError),
    InvalidRequest,
    InvalidUri(InvalidUri),
    InvalidResponse(String),
    WrongType(PsInterpretValueType, PsInterpretValueType),
    WrongMatchGroupValueType(PsInterpretValueType),
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for PsInterpretError {
    fn to_string(&self) -> String {
        match self {
            PsInterpretError::NoError => "No Error".to_string(),
            PsInterpretError::AssertionFailure(af) => {
                format!("Assertion failure: {}", af.to_string())
            }
            PsInterpretError::MelInterpreterError(meli) => {
                format!("MEL Interpreter error: {}", meli)
            }
            PsInterpretError::InvalidRequest => "Invalid HTTP request".to_string(),
            PsInterpretError::InvalidUri(iuri) => format!("Invalid URI: {iuri}"),
            PsInterpretError::InvalidResponse(response) => {
                format!("Invalid Response: {response}")
            }
            WrongType(expected, actual) => {
                format!(
                    "Wrong type: expected: {} actual: {}",
                    expected.to_string(),
                    actual.to_string()
                )
            }
            WrongMatchGroupValueType(actual) => {
                format!(
                    "Match group value should not have {} type",
                    actual.to_string()
                )
            }
        }
    }
}

#[derive(Debug)]
struct PsInterpreter<'a> {
    pub req: &'a mut dyn ProcessableRequestResponse,
}

impl<'a> PsInterpreter<'a> {
    #[allow(clippy::result_large_err)]
    fn scopes_from_req(&self) -> Result<Scopes<TypedValue>, PsInterpretError> {
        let mel_req = http::Request::builder()
            .uri(self.req.uri())
            .body("")
            .map_err(|_| PsInterpretError::InvalidRequest)?;

        Ok(Scopes::<TypedValue> {
            scopes: vec![Scope::<TypedValue>::from(mel_req)],
        })
    }

    #[allow(clippy::result_large_err)]
    fn evaluate_mel_expr(
        &self,
        expr: &Expr<Analyzed>,
        expected: Type,
    ) -> Result<TypedValue, PsInterpretError> {
        let expr_context = MelInterpContext {
            val: None,
            scopes: self.scopes_from_req()?,
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
        };

        let result = interpreter::interpret(expr, expr_context.clone())
            .map_err(PsInterpretError::MelInterpreterError)?
            .val
            .ok_or(PsInterpretError::AssertionFailure(
                PsInterpretAssertionFailures::MissingInterpreterExpressionValue,
            ))?;

        if result.tipe == expected {
            Ok(result)
        } else {
            Err(PsInterpretError::MelInterpreterError(
                MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        interpreter::interpret::MelInterpAssertion::TypeMismatch(
                            expected,
                            result.tipe,
                        ),
                    ),
                    context: expr_context.clone(),
                    location: expr.location().clone(),
                },
            ))
        }
    }
}

#[derive(Debug, Clone, Default)]
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

#[allow(clippy::to_string_trait_impl)]
impl ToString for PsInterpretValueType {
    fn to_string(&self) -> String {
        match &self {
            SyntheticResponse => "Synthetic response".to_string(),
            Terminate => "Terminate".to_string(),
            MatchResult => "Match result".to_string(),
            PsInterpretValueType::Header => "Header".to_string(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum PsInterpretValue {
    SyntheticResponse(http::Response<String>),
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

#[derive(Debug, Clone, Default)]
struct PsInterpretContext {
    mode: PsInterpretMode,
    result: Option<PsInterpretValue>,
}

impl PsInterpretContext {
    fn update_mode(&self, new_mode: PsInterpretMode) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.mode = new_mode;
        nc
    }
    fn update_result(&self, new_result: Option<PsInterpretValue>) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.result = new_result;
        nc
    }
}

impl<'a> PsVisitor<PsVerificationKey, PsInterpretContext, PsInterpretError> for PsInterpreter<'a> {
    fn visit_processing_stages(
        &mut self,
        _v: &TypedProcessingStages<PsVerificationKey>,
        _c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_rules(
        &mut self,
        v: &TypedStageRules<PsVerificationKey>,
        c: &PsInterpretContext,
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
            let result = self.evaluate_mel_expr(expr, Type::Boolean)?;
            match result {
                TypedValue {
                    tipe: Type::Boolean,
                    value: Value::Boolean(v),
                } => v,
                _ => unreachable!(),
            }
        } else {
            true
        };

        if doq {
            self.visit_stage_metadata(&v.value.stage_metadata, &c.update_result(Some(doq.into())))
        } else {
            Ok(c.update_result(Some(doq.into())))
        }
    }

    fn visit_expression_match(
        &mut self,
        _v: &TypedExpressionMatch<PsVerificationKey>,
        _c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_metadata(
        &mut self,
        v: &TypedStageMetadata<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        match &c.mode {
            PsInterpretMode::Request => {
                if let Some(req_xform) = &v.value.request_xform {
                    return self.visit_request_transform(req_xform, &c.clone());
                }
            }
            PsInterpretMode::Response => {
                if let Some(res_xform) = &v.value.response_xform {
                    return self.visit_response_transform(res_xform, &c.clone());
                }
            }
            _ => {
                return Err(PsInterpretError::AssertionFailure(
                    PsInterpretAssertionFailures::InvalidInterpreterMode,
                ));
            }
        }
        Ok(c.clone())
    }

    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(header_xform) = &v.value.xform {
            self.visit_header_transform(
                header_xform,
                &PsInterpretContext {
                    mode: PsInterpretMode::Request,
                    result: None,
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
                let result = self.evaluate_mel_expr(expr, Type::String)?;
                match result {
                    TypedValue {
                        tipe: Type::String,
                        value: Value::String(s),
                    } => Uri::try_from(s),
                    _ => unreachable!(),
                }
            } else {
                Uri::try_from(new_uri.clone())
            }
            .map_err(PsInterpretError::InvalidUri)?;
            self.req.set_uri(&new_uri);
        }

        Ok(c.clone())
    }

    fn visit_response_transform(
        &mut self,
        v: &TypedResponseTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(synthetic_response) = &v.value.synthetic {
            return self.visit_synthetic_response(synthetic_response, c);
        }

        if let Some(header_xform) = &v.value.xform {
            self.visit_header_transform(
                header_xform,
                &PsInterpretContext {
                    mode: PsInterpretMode::Request,
                    result: None,
                },
            )?;
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
                let result = self.evaluate_mel_expr(expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tipe: Type::Integer,
                        value: Value::Integer(i),
                    } => Ok(i),
                    _ => unreachable!(),
                }
            } else {
                new_response.parse::<i64>()
            }
            .map_err(|e| PsInterpretError::InvalidResponse(e.to_string()))?;
            self.req.set_response(&new_response);
        }

        Ok(c.clone())
    }

    fn visit_generic_metadata(
        &mut self,
        _v: &TypedGenericMetadata<PsVerificationKey>,
        _c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(to_delete) = &v.value.delete {
            for htr in to_delete {
                self.req.remove_header(htr);
            }
        }

        if let Some(to_add) = &v.value.add {
            for htr in to_add {
                self.visit_header(
                    htr,
                    &PsInterpretContext {
                        mode: PsInterpretMode::HeaderAdd,
                        ..Default::default()
                    },
                )?;
            }
        }

        if let Some(to_replace) = &v.value.replace {
            for htr in to_replace {
                self.visit_header(
                    htr,
                    &PsInterpretContext {
                        mode: PsInterpretMode::HeaderReplace,
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(c.clone())
    }

    fn visit_header(
        &mut self,
        v: &TypedHeader<PsVerificationKey>,
        c: &PsInterpretContext,
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
            let result = self.evaluate_mel_expr(expr, Type::String)?;
            match result {
                TypedValue {
                    tipe: Type::String,
                    value: Value::String(s),
                } => s,
                _ => unreachable!(),
            }
        } else {
            v.value.value.clone()
        };

        match c.mode {
            PsInterpretMode::HeaderAdd => {
                self.req.add_header(&v.value.name, &value);
                Ok(c.clone())
            }
            PsInterpretMode::HeaderReplace => {
                self.req.set_header_value(&v.value.name, &value);
                Ok(c.clone())
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
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut response = http::Response::builder();

        if let Some(headers) = &v.value.headers {
            for header in headers {
                let result = self.visit_header(header, &c.update_mode(HeaderCalculate))?;
                match result.result.ok_or(PsInterpretError::AssertionFailure(
                    PsInterpretAssertionFailures::MissingAnalyzedExpression,
                ))? {
                    PsInterpretValue::Header(name, value) => {
                        response.headers_mut().unwrap().insert(
                            HeaderName::from_bytes(name.as_bytes()).expect("Todo"),
                            HeaderValue::from_str(&value).expect("Todo"),
                        );
                    }
                    r => {
                        return Err(PsInterpretError::WrongType(
                            PsInterpretValueType::Header,
                            r.into(),
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
                let result = self.evaluate_mel_expr(expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tipe: Type::Integer,
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
                    let result = self.evaluate_mel_expr(expr, Type::String)?;
                    match result {
                        TypedValue {
                            tipe: Type::String,
                            value: Value::String(s),
                        } => s,
                        _ => unreachable!(),
                    }
                } else {
                    body.clone()
                }
            } else {
                "".to_string()
            })
            .expect("Could not create HTTP response");

        Ok(c.update_result(Some(PsInterpretValue::SyntheticResponse(response))))
    }

    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut result: Result<PsInterpretValue, PsInterpretError> = Err(
            PsInterpretError::AssertionFailure(PsInterpretAssertionFailures::MissingResult),
        );
        for mg in &v.value.match_groups {
            match self.visit_match_group(mg, &c.clone())?.result {
                Some(
                    r @ (PsInterpretValue::Terminate | PsInterpretValue::SyntheticResponse(_)),
                ) => {
                    // This result stops processing.
                    result = Ok(r);
                    break;
                }
                Some(r @ (PsInterpretValue::MatchYes | PsInterpretValue::MatchNo)) => {
                    // This result continues processing.
                    result = Ok(r);
                }
                Some(r) => {
                    return Err(WrongType(PsInterpretValueType::Terminate, r.into()));
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            }
        }
        Ok(c.update_result(Some(result?)))
    }

    fn visit_match_group(
        &mut self,
        v: &TypedMatchGroup<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let r = v
            .value
            .else_ifs
            .as_ref()
            .map(|else_ifs| else_ifs.iter())
            .unwrap_or([].iter());

        let mut result: Result<PsInterpretValue, PsInterpretError> = Err(
            PsInterpretError::AssertionFailure(PsInterpretAssertionFailures::MissingResult),
        );
        let rules = [&v.value.if_rule].into_iter().chain(r);
        for r in rules {
            match self.visit_stage_rules(r, &c.clone())?.result {
                Some(r @ PsInterpretValue::MatchNo) => {
                    result = Ok(r);
                    continue; // do the next rule.
                }
                Some(r) => {
                    result = Ok(r);
                    break; // do _not_ do the next rule in any other case.
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            };
        }
        Ok(c.update_result(Some(result?)))
    }
}

#[allow(clippy::result_large_err)]
pub fn interpret_stage(
    ts: &TypedStage<PsVerificationKey>,
    req: &mut dyn ProcessableRequestResponse,
    mode: PsInterpretMode,
) -> PsInterpretResult {
    let mut visitor = PsInterpreter { req };
    let context = PsInterpretContext {
        mode,
        ..Default::default()
    };

    match ts {
        TypedStage::ClientRequest(typed_client_request_stage) => {
            visitor.visit_client_request_stage(typed_client_request_stage, &context)
        }
        TypedStage::ClientResponse(_) => todo!(),
        TypedStage::OriginRequest(_) => todo!(),
        TypedStage::OriginResponse(_) => todo!(),
    }?
    .result
    .ok_or(PsInterpretError::AssertionFailure(
        PsInterpretAssertionFailures::MissingResult,
    ))
}

#[derive(Debug)]
enum EffectfulRequestActions {
    DeleteHeader(String),
    AddHeader(String, String),
    SetUri(String),
    SetResponse(i64),
}

#[derive(Debug, Default)]
struct EffectfulProcessableRequestResponse {
    log: Vec<EffectfulRequestActions>,
}

#[cfg(test)]
impl ProcessableRequestResponse for EffectfulProcessableRequestResponse {
    fn header_value(&self) -> &str {
        ""
    }

    fn headers(&self) -> &[&str] {
        &[]
    }

    fn set_header_value(&mut self, _header: &str, _value: &str) {}

    fn remove_header(&mut self, header: &str) {
        self.log
            .push(EffectfulRequestActions::DeleteHeader(header.to_string()));
    }

    fn uri(&self) -> Uri {
        Uri::default()
    }

    fn set_uri(&mut self, uri: &Uri) {
        self.log
            .push(EffectfulRequestActions::SetUri(uri.to_string()));
    }

    fn set_response(&mut self, response: &i64) {
        self.log
            .push(EffectfulRequestActions::SetResponse(*response));
    }

    fn add_header(&mut self, header: &str, value: &str) {
        self.log.push(EffectfulRequestActions::AddHeader(
            header.to_string(),
            value.to_string(),
        ));
    }
}

#[cfg(test)]
mod ps_interpreter_tests {
    use crate::{
        mel::{scope::Scopes, tvs::Type},
        ps::{
            interpret::{
                EffectfulProcessableRequestResponse, EffectfulRequestActions, PsInterpretMode,
                PsInterpretValue, interpret_stage,
            },
            spec::{TypedGenericStage, TypedStage},
            tests::test_helpers::{
                client_request_stage, expression_match, match_group, request_transform,
                response_transform, stage_metadata, synthetic_response, typed_header,
                typed_stage_rule,
            },
            verify::{PsVerifierContextValue, verifier, verify_ps_request_stage},
            visit::PsVisitor,
        },
        tests::read_test_file,
    };
    use std::assert_matches;
    use std::path::Path;

    #[test]
    fn test_interpret_client_request_stage() {
        let json = read_test_file(Path::new("./src/ps/tests/client_request_stage/if.json"));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");

        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(&result, &mut req, PsInterpretMode::Request)
            .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_response_header_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/client_request_stage/request_response_header_transform.json",
        ));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(&result, &mut req, PsInterpretMode::Request)
            .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example1.com/");
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example2.com/");
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        interpret_stage(
            &TypedStage::ClientRequest(value),
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(9));
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(10));
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(11));
        assert_matches!(result, PsInterpretValue::MatchYes);
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
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 0);
        assert_matches!(&result, PsInterpretValue::SyntheticResponse(r)
            if r.status() == 404 && r.body() == "This is a test." && r.headers().len() == 2);
    }
}
