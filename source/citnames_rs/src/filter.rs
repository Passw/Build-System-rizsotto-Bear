/*  Copyright (C) 2012-2023 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use json_compilation_db::Entry;

use crate::configuration::{Content, DuplicateFilterFields};

impl DuplicateFilterFields {
    fn hash_source(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        s.finish()
    }

    fn hash_source_and_output(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        entry.output.hash(&mut s);
        s.finish()
    }

    fn hash_all(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        entry.directory.hash(&mut s);
        entry.arguments.hash(&mut s);
        s.finish()
    }

    fn hash(&self) -> fn(&Entry) -> u64 {
        match self {
            DuplicateFilterFields::FileOnly =>
                DuplicateFilterFields::hash_source,
            DuplicateFilterFields::FileAndOutputOnly =>
                DuplicateFilterFields::hash_source_and_output,
            DuplicateFilterFields::All =>
                DuplicateFilterFields::hash_all,
        }
    }
}

type EntryFilterPredicate = Box<dyn FnMut(&Entry) -> bool>;

impl Into<EntryFilterPredicate> for DuplicateFilterFields {
    fn into(self) -> EntryFilterPredicate {
        let mut have_seen = HashSet::new();
        let hash_calculation = DuplicateFilterFields::hash(&self);

        Box::new(move |entry| {
            let hash = hash_calculation(&entry);
            if !have_seen.contains(&hash) {
                have_seen.insert(hash);
                true
            } else {
                false
            }
        })
    }
}

impl Into<EntryFilterPredicate> for Content {
    fn into(self) -> EntryFilterPredicate {
        let duplicates: EntryFilterPredicate = self.duplicate_filter_fields.into();

        Box::new(move |entry| {
            let source_check: EntryFilterPredicate = todo!();
            todo!()
        })
    }
}
