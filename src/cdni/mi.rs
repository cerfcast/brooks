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

//! All Metadata Information except for what is defined in Processing Stages.

use std::fmt::Debug;

/// Concrete instances of MI generated during interpretation that may be used to specify post-interpretation behavior.
pub trait MetadataInformationResultElement: Debug {}

#[derive(Debug, Default)]
pub struct MetadataInformationResultElements {
    pub elements: Vec<Box<dyn MetadataInformationResultElement>>,
}
