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

//! Logging (and related) metadata

use std::fmt::Display;

pub mod record;
pub mod recordi;
pub mod spec;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidRawValue(String /* Field */, String /* Value */),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidRawValue(field, value) => {
                write!(f, "{value} is an invalid value for {field} of a log record")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests;
