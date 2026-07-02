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

use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, Default)]
pub struct Scope<I: Clone + Default> {
    pub items: HashMap<String, I>,
}

impl<I: Clone + Default> Scope<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.items.get(id).cloned()
    }
    pub fn insert(&self, id: &str, value: I) -> Self {
        let mut next = self.items.clone();
        next.insert(id.to_string(), value);
        Self { items: next }
    }
}

#[derive(Debug, Clone)]
pub struct Scopes<I: Clone + Default> {
    pub scopes: Vec<Scope<I>>,
}

impl<I: Clone + Default> Scopes<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.scopes[0].lookup(id)
    }

    pub fn insert(&self, id: &str, value: I) -> Self {
        let updated_scope = self.scopes[0].insert(id, value);

        let mut next = self.scopes.clone();
        next[0] = updated_scope;

        Self { scopes: next }
    }

    pub fn enter(&self) -> Scopes<I> {
        let mut next = self.scopes.clone();
        next.extend([Scope::default()]);
        Self { scopes: next }
    }
}

impl<I: Clone + Default> Default for Scopes<I> {
    fn default() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }
}
