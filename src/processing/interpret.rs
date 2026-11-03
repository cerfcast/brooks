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

//! Interface for interpreting CDNI Metadata in a Particular Context

use std::fmt::Debug;

/// TODO
pub type InterpretationResult<Ck, Res, Error> = Result<(Ck, Res), Box<Error>>;

/// Interpret a validated representation of a given piece of Metadata Information.
pub trait Interpreter<Ck, Res, Interest, Error>: Debug {
    /// Perform the semantics of the MI.
    fn interpret(&self, input: Ck) -> InterpretationResult<Ck, Res, Error>;

    fn at(&self, interest: Interest) -> bool;
}
