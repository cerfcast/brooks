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

use crate::cdni::{
    ps::spec::{
        TypedClientRequestStage, TypedClientResponseStage, TypedExpressionMatch, TypedHeader,
        TypedHeaderTransform, TypedMatchGroup, TypedOriginRequestStage, TypedOriginResponseStage,
        TypedProcessingStages, TypedRequestTransform, TypedResponseTransform, TypedStageMetadata,
        TypedStageRules, TypedSyntheticResponse,
    },
    spec::TypedGenericMetadata,
};

use std::fmt::Debug;

pub type PsVisitorResult<T, E> = Result<T, E>;
pub trait PsVisitor<A: Debug + Clone + Default, C, O, E> {
    fn visit_processing_stages(
        &mut self,
        v: &TypedProcessingStages<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_stage_rules(&mut self, v: &TypedStageRules<A>, c: C) -> PsVisitorResult<O, E>;
    fn visit_expression_match(
        &mut self,
        v: &TypedExpressionMatch<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_stage_metadata(&mut self, v: &TypedStageMetadata<A>, c: C) -> PsVisitorResult<O, E>;
    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_response_transform(
        &mut self,
        v: &TypedResponseTransform<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_generic_metadata(
        &mut self,
        v: &TypedGenericMetadata<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_header(&mut self, v: &TypedHeader<A>, c: C) -> PsVisitorResult<O, E>;
    fn visit_synthetic_response(
        &mut self,
        v: &TypedSyntheticResponse<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_client_response_stage(
        &mut self,
        v: &TypedClientResponseStage<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_origin_request_stage(
        &mut self,
        v: &TypedOriginRequestStage<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_origin_response_stage(
        &mut self,
        v: &TypedOriginResponseStage<A>,
        c: C,
    ) -> PsVisitorResult<O, E>;
    fn visit_match_group(&mut self, v: &TypedMatchGroup<A>, c: C) -> PsVisitorResult<O, E>;
}
