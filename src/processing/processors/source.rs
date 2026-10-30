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

//! The Source Generic Metadata Processing Implementation

use reqwest::dns::{Addrs, Resolve};

use crate::{
    cdni::original::spec::TypedSource,
    processing::{
        interpret::{
            self, Error, InterpretationResult, MetadataInformationResultElements,
            ProcessedRequestResponse, Stages,
        },
        processors::{
            SimpleProcessorsInterpreterContext,
            ps::{self, PsInterpretValue},
        },
    },
    prr::{self, Prr},
    verifying::original::verify::SourceVerificationKey,
};

use std::fmt::Debug;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct SourceMetadataResolver {
    pub addr: SocketAddr,
}

impl Resolve for SourceMetadataResolver {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let addrs: Addrs = Box::new(vec![self.addr].into_iter());
        Box::pin(std::future::ready(Ok(addrs)))
    }
}

impl
    interpret::Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (ps::PsInterpretValue, MetadataInformationResultElements),
        Error,
    > for TypedSource<SourceVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (ps::PsInterpretValue, MetadataInformationResultElements),
        Error,
    > {
        let req = &*input.req;

        let req: reqwest::Request = req
            .try_into()
            .map_err(|e: prr::Error| Error::InvalidInput(e.into()))?;

        let req_method = req.method().clone();
        let req_url = req.url().clone();

        let mut clientb = reqwest::Client::builder();

        let resolver = SourceMetadataResolver {
            addr: *self
                .value
                .aug
                .endpoints
                .first()
                .ok_or(Error::AssertionFailure(
                    "Missing endpoints".to_string().into(),
                ))?,
        };

        clientb = clientb.dns_resolver(resolver);

        let client = clientb.build().map_err(|e| Error::RuntimeError(e.into()))?;

        let result: Result<ProcessedRequestResponse, Error> = input.runtime.block_on(async {
            let r = client
                .execute(req)
                .await
                .map_err(|e| Error::RuntimeError(e.into()))?;

            let mut result = ProcessedRequestResponse::new_response(&req_url, &req_method);

            result.oheaders = r.headers().clone();

            result
                .set_status(&r.status().as_u16())
                .map_err(|_| Error::RuntimeError(prr::Error::BadStatus.into()))?;

            let b = r.bytes().await.map_err(|e| Error::RuntimeError(e.into()))?;
            result.obody = b.to_vec();
            Ok(result)
        });

        Ok((
            input.with_res(Box::new(result?)),
            (
                PsInterpretValue::MatchNo,
                MetadataInformationResultElements::default(),
            ),
        ))
    }

    fn at(&self, stage: Stages) -> bool {
        stage == Stages::Source
    }
}
