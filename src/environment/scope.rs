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

use std::{collections::HashMap, fmt::Debug, ops::Add};

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

impl<I: Clone + Default> Add for &Scope<I> {
    type Output = Scope<I>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut ns = Scope::<I> {
            items: HashMap::new(),
        };
        for (k, v) in &self.items {
            ns = ns.insert(k, v.clone());
        }
        for (k, v) in &rhs.items {
            ns = ns.insert(k, v.clone());
        }
        ns
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

    pub fn current(&self) -> &Scope<I> {
        &self.scopes[0]
    }
}

impl<I: Clone + Default> Default for Scopes<I> {
    fn default() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }
}

#[cfg(test)]
mod scope_tests {
    use std::assert_matches;
    use std::collections::HashMap;

    use crate::environment::scope::Scope;

    #[test]
    fn test_operator_plus() {
        let mut s1 = Scope::<i8> {
            items: HashMap::new(),
        };
        s1 = s1.insert("x", 5);
        let mut s2 = Scope::<i8> {
            items: HashMap::new(),
        };
        s2 = s2.insert("y", 4);

        let s3 = &s1 + &s2;

        assert_matches!(s3.lookup("y"), Some(4));
        assert_matches!(s3.lookup("x"), Some(5));
    }
}
