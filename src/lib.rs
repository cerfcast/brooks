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

#[macro_use]
pub mod logging;
#[allow(dead_code)]
pub mod common;
#[allow(dead_code)]
pub mod utils;

#[allow(dead_code)]
#[cfg(feature = "mel")]
pub mod mel;

#[allow(dead_code)]
#[cfg(feature = "verifying")]
pub mod verifying;

#[allow(dead_code)]
#[cfg(feature = "processing")]
pub mod processing;

#[allow(dead_code)]
pub mod visitors;

#[allow(dead_code)]
pub mod cdni;

#[allow(dead_code)]
pub mod integrations;

#[allow(dead_code)]
pub mod environment;

#[allow(dead_code)]
pub mod prr;

#[allow(dead_code)]
pub mod macros;

#[cfg(test)]
pub(crate) mod tests;
