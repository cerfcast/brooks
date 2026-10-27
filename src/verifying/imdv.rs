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

//! Verification of [Interconnection Metadata](https://datatracker.ietf.org/doc/rfc8006/)

use crate::verifying::mel::verify::SetVariableMetadataAnalyzer;
use crate::verifying::verify::{
    CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError, Verifier,
};
use crate::{environment::scope::Scopes, mel::types::Type};

use crate::verifying::original::verify::SourceMetadataAnalyzer;
use crate::verifying::ps::verify::PsMetadataAnalyzer;

use crate::cdni::{gmd::spec::TypedGenericMetadata, imd::spec::HostMetadata};

/// Verify a piece of [Interconnection Metadata](https://datatracker.ietf.org/doc/rfc8006/).
///
/// TODO: Make clear in the documentation that this function is a _major_ entry point for library clients.
pub fn verify_interconnection_metadata(
    metadata: &HostMetadata<()>,
    scopes: Scopes<Type>,
) -> Result<(HostMetadata<CdniVerificationKey>, Scopes<Type>), Box<MetadataVerificationError>> {
    let mut analyzed_md_elements: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];

    let processors: Vec<&dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>> = vec![
        &PsMetadataAnalyzer {} as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>,
        &SourceMetadataAnalyzer {}
            as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>,
        &SetVariableMetadataAnalyzer {}
            as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>,
    ];

    let mut analysis_context = MetadataAnalysisContext { scopes };
    for md in &metadata.metadata {
        let mut no_processor = true;
        for processor in &processors {
            if !processor.type_name(&md.tpe) {
                continue;
            }

            no_processor = false;

            let (new_analysis_context, analyzed_md) = processor.analyze(md, analysis_context)?;

            analysis_context = new_analysis_context;

            analyzed_md_elements.push(TypedGenericMetadata {
                tpe: md.tpe.clone(),
                value: md.value.clone(),
                aug: analyzed_md,
            });
        }

        if no_processor {
            return Err(MetadataVerificationError::NoProcessor(md.tpe.clone()).into());
        }
    }

    Ok((
        HostMetadata {
            metadata: analyzed_md_elements,
            aug: Default::default(),
        },
        analysis_context.scopes,
    ))
}
