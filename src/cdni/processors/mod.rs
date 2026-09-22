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

//! Processors for Metadata Information.

use crate::{
    cdni::ps::interpret::PsInterpretMode,
    environment::scope::{Scope, Scopes},
    mel::{interpreter::interpret::TypedValue, types::Type},
    tools::prr,
};

#[cfg(feature = "mi_source")]
pub mod source;

#[cfg(feature = "mi_ps")]
pub mod ps;

#[derive(Debug, Clone, Default)]
pub struct SimpleProcessorsAnalysisContext {
    pub scopes: Scopes<Type>,
}

#[derive(Debug)]
pub struct SimpleProcessorsInterpreterContext<'a> {
    pub scope: Option<&'a Scope<TypedValue>>,
    pub mode: PsInterpretMode,
    pub runtime: &'a tokio::runtime::Runtime,
    pub rr: Box<dyn prr::Prr<Vec<u8>>>,
}

impl<'a> SimpleProcessorsInterpreterContext<'a> {
    /// Create an interpreter context based on an existing one, but with a new RR.
    pub fn with_new_rr(self, new_rr: Box<dyn prr::Prr<Vec<u8>>>) -> Self {
        Self {
            scope: self.scope,
            mode: match new_rr.tpe() {
                prr::PrrType::Request => PsInterpretMode::Request,
                prr::PrrType::Response => PsInterpretMode::Response,
            },
            runtime: self.runtime,
            rr: new_rr,
        }
    }
}
