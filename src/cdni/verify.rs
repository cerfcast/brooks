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

use std::fmt::Display;

use crate::{
    cdni::spec::{HostMetadata, TypedGenericMetadata},
    mel::{scope::Scopes, tvs::Type},
    ps::{
        spec::{TypedGenericStage, TypedStage},
        verify::{PsVerificationError, PsVerificationKey, verify_ps_request_stage},
    },
};

#[derive(Debug, Clone, Default)]
pub struct HostMetadataVerificationKey {
    pub stage: Option<TypedStage<PsVerificationKey>>,
}

#[derive(Debug, Clone, Default)]
pub enum HostMetadataVerificationError {
    #[default]
    NoError,
    NotProcessingStage,
    PsVerificationError(PsVerificationError),
}

impl Display for HostMetadataVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostMetadataVerificationError::NoError => write!(f, "No error!"),
            HostMetadataVerificationError::NotProcessingStage => {
                write!(f, "Value was not a processing stage!")
            }
            HostMetadataVerificationError::PsVerificationError(pse) => write!(f, "{pse}"),
        }
    }
}

pub fn verify_host_metadata(
    metadata: &HostMetadata<()>,
    scopes: Scopes<Type>,
) -> Result<HostMetadata<HostMetadataVerificationKey>, HostMetadataVerificationError> {
    let mut stages: Vec<TypedGenericMetadata<HostMetadataVerificationKey>> = vec![];
    for md in &metadata.metadata {
        let generic_stage =
            serde_json::from_str::<TypedGenericStage>(&serde_json::to_string(md).expect("Ahhh"))
                .map_err(|_| HostMetadataVerificationError::NotProcessingStage)?;
        let verified_generic_stage = verify_ps_request_stage(&generic_stage, scopes.clone())
            .map_err(HostMetadataVerificationError::PsVerificationError)?;

        stages.push(TypedGenericMetadata {
            tpe: md.tpe.clone(),
            value: md.value.clone(),
            aug: HostMetadataVerificationKey {
                stage: Some(verified_generic_stage),
            },
        });
    }

    Ok(HostMetadata {
        metadata: stages,
        aug: Default::default(),
    })
}
