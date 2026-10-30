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
    environment::scope::Scopes, mel::interpreter::values::TypedValue, processing::imdi, prr,
};

pub mod cors;
pub mod mel;
pub mod ps;
pub mod source;

#[derive(Debug)]
pub struct SimpleProcessorsInterpreterContext<'a> {
    pub scopes: Scopes<TypedValue>,
    pub mode: imdi::InterpretMode,
    pub runtime: &'a tokio::runtime::Handle,
    pub req: Box<dyn prr::Prr<Vec<u8>>>,
    pub res: Option<Box<dyn prr::Prr<Vec<u8>>>>,
}

impl<'a> SimpleProcessorsInterpreterContext<'a> {
    /// Create an interpreter context based on an existing one, but with a new RR.
    pub fn with_res(self, res: Box<dyn prr::Prr<Vec<u8>>>) -> Self {
        Self {
            scopes: self.scopes,
            mode: match res.tpe() {
                prr::PrrType::Request => imdi::InterpretMode::Request,
                prr::PrrType::Response => imdi::InterpretMode::Response,
            },
            runtime: self.runtime,
            req: self.req,
            res: Some(res),
        }
    }

    pub fn with_new_value_in_scope(self, name: &str, new_value: TypedValue) -> Self {
        Self {
            scopes: self.scopes.insert(name, new_value),
            mode: self.mode,
            runtime: self.runtime,
            req: self.req,
            res: self.res,
        }
    }
}
