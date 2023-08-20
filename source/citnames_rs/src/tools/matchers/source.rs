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

use std::collections::HashSet;
use lazy_static::lazy_static;

pub fn looks_like_a_source_file(argument: &str) -> bool {
    // unix flags
    if argument.starts_with("-") {
        return false;
    }
    // windows flags
    if argument.starts_with("/") {
        return false;
    }
    if let Some((_, extension)) = argument.rsplit_once('.') {
        return EXTENSIONS.contains(extension);
    }
    false
}

lazy_static! {
    static ref EXTENSIONS: HashSet<&'static str> = {
        let mut set = HashSet::new();

        // header files
        set.insert("h");
        set.insert("hh");
        set.insert("H");
        set.insert("hp");
        set.insert("hxx");
        set.insert("hpp");
        set.insert("HPP");
        set.insert("h++");
        set.insert("tcc");
        // C
        set.insert("c");
        set.insert("C");
        // C++
        set.insert("cc");
        set.insert("CC");
        set.insert("c++");
        set.insert("C++");
        set.insert("cxx");
        set.insert("cpp");
        set.insert("cp");
        // CUDA
        set.insert("cu");
        // ObjectiveC
        set.insert("m");
        set.insert("mi");
        set.insert("mm");
        set.insert("M");
        set.insert("mii");
        // Preprocessed
        set.insert("i");
        set.insert("ii");
        // Assembly
        set.insert("s");
        set.insert("S");
        set.insert("sx");
        set.insert("asm");
        // Fortran
        set.insert("f");
        set.insert("for");
        set.insert("ftn");
        set.insert("F");
        set.insert("FOR");
        set.insert("fpp");
        set.insert("FPP");
        set.insert("FTN");
        set.insert("f90");
        set.insert("f95");
        set.insert("f03");
        set.insert("f08");
        set.insert("F90");
        set.insert("F95");
        set.insert("F03");
        set.insert("F08");
        // go
        set.insert("go");
        // brig
        set.insert("brig");
        // D
        set.insert("d");
        set.insert("di");
        set.insert("dd");
        // Ada
        set.insert("ads");
        set.insert("abd");

        set.shrink_to_fit();
        set
    };
}

#[cfg(test)]
mod test {
    use crate::tools::matchers::source::looks_like_a_source_file;

    #[test]
    fn test_filenames() {
        assert!(looks_like_a_source_file("source.c"));
        assert!(looks_like_a_source_file("source.cpp"));
        assert!(looks_like_a_source_file("source.cxx"));
        assert!(looks_like_a_source_file("source.cc"));

        assert!(looks_like_a_source_file("source.h"));
        assert!(looks_like_a_source_file("source.hpp"));

        assert!(!looks_like_a_source_file("gcc"));
        assert!(!looks_like_a_source_file("clang"));
        assert!(!looks_like_a_source_file("-o"));
        assert!(!looks_like_a_source_file("-Wall"));
        assert!(!looks_like_a_source_file("/o"));
    }
}