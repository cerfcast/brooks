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

// This file is auto-generated. Use ./tools/logging_record_gen.py to update.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net;

/*
"c-" refers to the User Agent that issues the request (corresponds to the "client" of W3C Extended Log Format)
"d-" refers to the dCDN (relative to a given CDN acting as an uCDN)
"s-" refers to the dCDN Surrogate that serves the request (corresponds to the "server" of the W3C Extended Log Format)
"u-" refers to the uCDN (relative to a given CDN acting as a dCDN)
"cs-" refers to communication from the User Agent towards the dCDN Surrogate
"sc-" refers to communication from the dCDN Surrogate towards the User Agent
*/
