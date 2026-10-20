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
    mel::{self, interpreter::interpret::MelInterpContext},
    processing::{
        interpret::{
            self,
            Error::{self, RuntimeError},
            InterpretationResult, MetadataInformationResultElements, Stages,
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

            let output = input.with_new_value_in_scope(
                &self.variable_name,
                result.val.ok_or(RuntimeError(
                    "Missing value after successful interpretation of Mel expression".into(),
                ))?,
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

    fn stage(&self, stage: Stages) -> bool {
        stage != Stages::Source
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
            interpreter::values::{TypedValue, Value},
            scope::builtin_function_types,
            types::Type,
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
                    scopes: builtin_function_types().into(),
                },
            )
            .expect("Could not analyze correct MI.SetVariable");

        let svmi = match result.1 {
            CdniVerificationKey::SetVariable(typed_set_variable) => typed_set_variable,
            _ => panic!("Result of verifying correct MI.SetVariable was of incorrect type"),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Could not get a runtime to use for testing interpret function");
        let result = svmi
            .value
            .interpret(SimpleProcessorsInterpreterContext {
                scopes: Scopes::<TypedValue>::default(),
                mode: PsInterpretMode::None,
                runtime: rt.handle(),
                rr: Box::new(ProcessedRequestResponse::new_request(
                    &Url::from_str("http://www.example.com/")
                        .expect("Could not get a URI from a valid HTTP address"),
                    &http::Method::GET,
                )),
            })
            .expect("Could not interpret correct MI.SetVariable");

        let value = result
            .0
            .scopes
            .lookup("is_bad")
            .expect("Could not get interpreted value by name from resulting scope");
        assert_matches!(
            value,
            TypedValue {
                value: Value::Boolean(true),
                tpe: Type::Boolean
            }
        );
    }
}
