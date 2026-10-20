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

use crate::verifying::mel::verify::SetVariableMetadataAnalyzer;
use crate::verifying::verify::{
    CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError, Verifier,
};
use crate::{environment::scope::Scopes, mel::types::Type};

use crate::verifying::original::verify::SourceMetadataAnalyzer;
use crate::verifying::ps::verify::PsMetadataAnalyzer;

use crate::cdni::{gmd::spec::TypedGenericMetadata, imd::spec::HostMetadata};

/// Verify given Interconnection Metadata for the given request.
///
/// TODO: Make clear in the documentation that this function is a _major_ entry point for library clients.
pub fn verify_interconnection_metadata(
    metadata: &HostMetadata<()>,
    scopes: Scopes<Type>,
) -> Result<HostMetadata<CdniVerificationKey>, Box<MetadataVerificationError>> {
    let mut stages: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];

    #[allow(clippy::vec_init_then_push)]
    #[allow(unused_mut)]
    let mut processors: Vec<&dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>> =
        vec![];

    #[allow(clippy::vec_init_then_push)]
    processors
        .push(&PsMetadataAnalyzer {}
            as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>);

    #[allow(clippy::vec_init_then_push)]
    processors.push(&SourceMetadataAnalyzer {}
        as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>);

    #[allow(clippy::vec_init_then_push)]
    processors.push(&SetVariableMetadataAnalyzer {}
        as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>);

    for md in &metadata.metadata {
        let mut no_processor = true;
        for processor in &processors {
            if !processor.type_name(&md.tpe) {
                continue;
            }

            no_processor = false;

            let (_, analyzed) = processor.analyze(
                md,
                MetadataAnalysisContext {
                    scopes: scopes.clone(),
                },
            )?;

            stages.push(TypedGenericMetadata {
                tpe: md.tpe.clone(),
                value: md.value.clone(),
                aug: analyzed,
            });
        }

        if no_processor {
            return Err(MetadataVerificationError::NoProcessor(md.tpe.clone()).into());
        }
    }

    Ok(HostMetadata {
        metadata: stages,
        aug: Default::default(),
    })
}
