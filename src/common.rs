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

use std::fmt::Debug;
use std::fmt::Display;

pub trait Location: Display + Debug {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLocation {
    pub start: usize,
    pub extent: usize,
}

impl Location for TextLocation {}

impl Display for TextLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} to {}", self.start, self.start + self.extent)
    }
}

pub trait Formatter<T> {
    fn format(&self, value: &T) -> String;
}

pub trait Pretty {
    fn pretty(&self, formatter: &dyn Formatter<Self>) -> String;
}

impl<I> Pretty for I {
    fn pretty(&self, formatter: &dyn Formatter<I>) -> String {
        formatter.format(self)
    }
}
