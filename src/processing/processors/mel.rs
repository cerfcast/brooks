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
    cdni::mel::spec::SetVariable,
    logging::{LogLevel::Debug, LogMsgs},
    mel::{
        self, MEL_USER_SET_VARIABLE_STRUCT_NAME,
        interpreter::{
            interpret::MelInterpContext,
            values::{TypedValue, Value},
        },
        types::{Struct, Type},
    },
    processing::{
        interpret::{
            self,
            Error::{self, InvalidType, RuntimeError},
            InterpretationResult, MetadataInformationResultElements,
        },
        processors::{
            SimpleProcessorsInterpreterContext,
            ps::{self, PsInterpretValue::MatchYes},
        },
    },
    verifying::mel::verify::SetVariableVerificationKey,
};

impl
    interpret::Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (ps::PsInterpretValue, MetadataInformationResultElements),
        Error,
    > for SetVariable<SetVariableVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (ps::PsInterpretValue, MetadataInformationResultElements),
        Error,
    > {
        let lmsgs = LogMsgs::new(Debug);

        // First, try to get the var struct from the scope.
        let var = input
            .scopes
            .lookup(MEL_USER_SET_VARIABLE_STRUCT_NAME)
            .ok_or(RuntimeError("var not in scope".into()))?;

        let (mut var, var_tpe) = match var {
            TypedValue {
                value: Value::Struct(value),
                tpe: Type::Struct(tpe),
            } => (value, tpe),
            TypedValue { value: _, tpe } => {
                return Err(InvalidType(
                    Type::Struct(Struct::new(MEL_USER_SET_VARIABLE_STRUCT_NAME)).to_string(),
                    tpe.to_string(),
                )
                .into());
            }
        };

        if let Some(expression) = &self.aug.expression {
            let result = mel::interpreter::interpret(
                expression,
                MelInterpContext {
                    val: None,
                    scopes: input.scopes.clone(),
                    log: lmsgs,
                },
            )
            .map_err(|e| RuntimeError(Box::new(e)))?;

            var.insert_field(
                &self.variable_name,
                result.val.ok_or(RuntimeError(
                    "Missing value after successful interpretation of Mel expression".into(),
                ))?,
            )
            .map_err(|e| RuntimeError(e))?;

            let output = input.with_new_value_in_scope(
                MEL_USER_SET_VARIABLE_STRUCT_NAME,
                TypedValue {
                    value: Value::Struct(var),
                    tpe: Type::Struct(var_tpe),
                },
            );

            Ok((
                output,
                (MatchYes, MetadataInformationResultElements::default()),
            ))
        } else if let Some(_value) = &self.aug.value {
            todo!()
        } else {
            todo!()
        }
    }
}

#[cfg(test)]
mod test {
    use std::{assert_matches, str::FromStr};

    use url::Url;

    use crate::{
        cdni::{
            mel::spec::TypedSetVariable,
            tests::test_helpers::{generic_metadata, set_variable},
        },
        environment::scope::Scopes,
        mel::{
            MEL_USER_SET_VARIABLE_STRUCT_NAME,
            interpreter::values::{TypedValue, Value, mel_var_value_from_type},
            types::{Type, builtin_function_types, minimal_core_variable_types},
        },
        processing::{
            interpret::{Interpreter, ProcessedRequestResponse},
            processors::{SimpleProcessorsInterpreterContext, ps::PsInterpretMode},
        },
        verifying::{
            mel::verify::SetVariableMetadataAnalyzer,
            verify::{CdniVerificationKey, MetadataAnalysisContext, Verifier},
        },
    };

    #[test]
    fn verify_test() {
        let variable = set_variable("is_bad".to_string(), "true".to_string(), Some(true));

        let json = generic_metadata(
            &TypedSetVariable::<()>::typed_cdni_metadata_name(),
            Some(
                serde_json::to_value(variable.value)
                    .expect("Could not convert valid SetVariable to Value"),
            ),
        );

        let svma = SetVariableMetadataAnalyzer {};

        let result = svma
            .analyze(
                &json,
                MetadataAnalysisContext {
                    scopes: (&builtin_function_types() + &minimal_core_variable_types()).into(),
                },
            )
            .expect("Could not analyze correct MI.SetVariable");

        let svmi = match result.1 {
            CdniVerificationKey::SetVariable(typed_set_variable) => typed_set_variable,
            _ => panic!("Result of verifying correct MI.SetVariable was of incorrect type"),
        };

        let var = result
            .0
            .scopes
            .lookup(MEL_USER_SET_VARIABLE_STRUCT_NAME)
            .expect("Could not get the var struct from the types of the analyzed generic metadata");

        let var = match var {
            Type::Struct(s) => s,
            _ => panic!("var had the wrong type in the analyzed generic metadata"),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Could not get a runtime to use for testing interpret function");
        let result = svmi
            .value
            .interpret(SimpleProcessorsInterpreterContext {
                scopes: Scopes::<TypedValue>::default().insert(
                    MEL_USER_SET_VARIABLE_STRUCT_NAME,
                    mel_var_value_from_type(var),
                ),
                mode: PsInterpretMode::None,
                runtime: rt.handle(),
                rr: Box::new(ProcessedRequestResponse::new_request(
                    &Url::from_str("http://www.example.com/")
                        .expect("Could not get a URI from a valid HTTP address"),
                    &http::Method::GET,
                )),
            })
            .expect("Could not interpret correct MI.SetVariable");

        let value = match result
            .0
            .scopes
            .lookup(MEL_USER_SET_VARIABLE_STRUCT_NAME)
            .unwrap_or_else(|| {
                panic!(
                    "Could not get {} from resulting scope",
                    MEL_USER_SET_VARIABLE_STRUCT_NAME
                )
            }) {
            TypedValue {
                value: Value::Struct(s),
                tpe: Type::Struct(_),
            } => s.fields.get("is_bad").cloned(),
            _ => panic!(
                "{} is not a struct in resulting scope",
                MEL_USER_SET_VARIABLE_STRUCT_NAME
            ),
        };

        let value = match value {
            Some(v) => v,
            None => panic!(
                "is_bad field of {} does not have a value in resulting scope",
                MEL_USER_SET_VARIABLE_STRUCT_NAME
            ),
        };
        assert_matches!(value, TypedValue{ value: Value::Boolean(v), tpe: _} if v)
    }
}
