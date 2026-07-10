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

use crate::ps::spec::{
    TypedExpressionMatch, TypedGenericMetadata, TypedHeader, TypedHeaderTransform,
    TypedProcessingStages, TypedRequestTransform, TypedResponseTransform, TypedStageMetadata,
    TypedStageRules, TypedSyntheticResponse,
};

use std::fmt::Debug;

pub type CdniVisitorResult<T, E> = Result<T, E>;
pub trait CdniVisitor<A: Debug + Clone + Default, O, E> {
    fn visit_processing_stages(
        &self,
        v: &TypedProcessingStages<A>,
        c: &O,
    ) -> CdniVisitorResult<O, E>;
    fn visit_stage_rules(&self, v: &TypedStageRules<A>, c: &O) -> CdniVisitorResult<O, E>;
    fn visit_expression_match(&self, v: &TypedExpressionMatch<A>, c: &O)
    -> CdniVisitorResult<O, E>;
    fn visit_stage_metadata(&self, v: &TypedStageMetadata<A>, c: &O) -> CdniVisitorResult<O, E>;
    fn visit_request_transform(
        &self,
        v: &TypedRequestTransform<A>,
        c: &O,
    ) -> CdniVisitorResult<O, E>;
    fn visit_response_transform(
        &self,
        v: &TypedResponseTransform<A>,
        c: &O,
    ) -> CdniVisitorResult<O, E>;
    fn visit_generic_metadata(&self, v: &TypedGenericMetadata<A>, c: &O)
    -> CdniVisitorResult<O, E>;
    fn visit_header_transform(&self, v: &TypedHeaderTransform<A>, c: &O)
    -> CdniVisitorResult<O, E>;
    fn visit_header(&self, v: &TypedHeader<A>, c: &O) -> CdniVisitorResult<O, E>;
    fn visit_synthetic_response(
        &self,
        v: &TypedSyntheticResponse<A>,
        c: &O,
    ) -> CdniVisitorResult<O, E>;
}
