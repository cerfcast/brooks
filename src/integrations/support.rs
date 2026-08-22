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

#[allow(clippy::missing_safety_doc)]
pub unsafe fn to_null_terminated_str(s: &str) -> Vec<u8> {
    let mut res = vec![0u8; s.len() + 1];
    for (idx, l) in s.as_bytes().iter().enumerate() {
        res[idx] = *l;
    }

    res
}

#[allow(unused_macros)]
macro_rules! c_str_literal {
    ($lit:expr) => {
        CStr::from_bytes_with_nul(concat!($lit, "\0").as_bytes())
            .expect("Could not create c string from literal")
            .as_ptr()
    };
}
