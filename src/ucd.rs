use crate::normalise::Normalisation;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

// The simplest way to get them is to extract them from the XML, because otherwise they're spread
// out over two files, DerivedNormalizationProps and UCDData. But actually parsing the XML is a
// nightmare. So I'm extracting the xml (grep) to json files.
// These are not intended to be highly optimised, that's its own rabbit hole.

lazy_static! {
    // combining marks
    static ref NFC_QC_M: HashSet<u32> = {
        let f = std::fs::File::open("resources/nfc-quick-check-maybe.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

     // composition exclusions
    static ref NFC_QC_N: HashSet<u32> = {
        let f = std::fs::File::open("resources/nfc-quick-check-no.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    // cat ucd.all.flat.xml | grep 'NFD_QC="N"' | grep -Eo 'cp="([0-9A-F]+)"'
    static ref NFD_QC_N: HashSet<u32> = {
        let f = std::fs::File::open("resources/nfd-quick-check-no.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    // D114 For any given version of the Unicode Standard, the list of primary composites
    // can be computed by extracting all canonical decomposable characters (dt=can) from
    // UnicodeData.txt in the Unicode Character Database, adding the list of precom-
    // posed Hangul syllables (D132), and subtracting the list of Full Composition
    // Exclusions.
    // cat ucd.all.flat.xml | grep 'dt="can"' | grep -v 'Comp_Ex="Y"'
    static ref PRIMARY_COMPOSITES: HashMap<[u32; 2], u32> = {
        let f = std::fs::File::open("resources/primary-composites.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        let pairs: Vec<([u32; 2], u32)> = serde_json::from_reader(rdr).unwrap();
        HashMap::from_iter(pairs.into_iter())
    };

    // Canonical (dt=can) decomposition mappings. Unlike the composite mappings, they include
    // the composition exclusions.
    // cat ucd.all.flat.xml | grep 'dt="can"'
    static ref DECOMPOSITION_MAPPINGS: HashMap<u32, Vec<u32>> = {
        let f = std::fs::File::open("resources/decomposition-mappings.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    static ref COMBINING_CLASSES: HashMap<u32, u8> = {
        let f = std::fs::File::open("resources/combining-class.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    // Upper = is uppercase , Lower = is lowercase, OUpper/Olower = Other_*Case
    // su/l/tc = simple upper/lower/title case mappings, excluding special cases
    // u/l/c = full case mappings
    // cat dev/ucd.all.flat.xml | grep -v ' uc="#"' | grep ' uc=' (make sure to get space before uc, don't match suc)
    static ref UPPERCASE_MAPPINGS: HashMap<u32, Vec<u32>> = {
        let f = std::fs::File::open("resources/uppercase-mappings.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    // excludes 0130, which is the one whose lc is two code points
    static ref LOWERCASE_MAPPINGS: HashMap<u32, u32> = {
        let f = std::fs::File::open("resources/lowercase-mappings.json").unwrap();
        let rdr = std::io::BufReader::new(f);
        serde_json::from_reader(rdr).unwrap()
    };

    // grep 'Cased="Y"' | grep 'Cased='
    static ref CASED: HashSet<u32> = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new("resources/cased.json")).unwrap()
    ).unwrap();

    // grep 'CI="Y"' | grep 'CI='
    static ref CASE_IGNORABLE: HashSet<u32> = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new("resources/case-ignorable.json")
    ).unwrap()).unwrap();

    // cat ucd.all.flat.xml | grep -v ' cf="#"' | grep ' cf=' (don't want sfc, simple case folding)
    static ref FULL_CASE_FOLDING: HashMap<u32, Vec<u32>> = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new("resources/case-folding.json")
    ).unwrap()).unwrap();

    // Doesn't include codepoints whose value is XX, because there are 131350 of those, whereas the
    // rest only total 14190.
    // grep 'GCB='
    static ref GRAPHEME_CLUSTER_BREAK: HashMap<u32, GraphemeClusterBreak> = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new("resources/grapheme-cluster-break.json")
    ).unwrap()).unwrap();

     // grep 'ExtPict="Y"'
    static ref EXTENDED_PICTORIAL: HashSet<u32> = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new("resources/extended-pictorial.json")
    ).unwrap()).unwrap();

}

pub fn decomposition_mapping(code_point: u32) -> Option<Vec<u32>> {
    DECOMPOSITION_MAPPINGS
        .get(&code_point)
        .map(|mapping| mapping.clone())
}

pub fn combining_class(code_point: u32) -> u8 {
    COMBINING_CLASSES
        .get(&code_point)
        .map(|ccc| *ccc)
        .unwrap_or(0)
}

pub fn is_starter(code_point: u32) -> bool {
    combining_class(code_point) == 0
}

pub enum QuickCheckVal {
    Yes,
    No,
    Maybe,
}

pub fn is_allowed(code_point: u32, normalisation: &Normalisation) -> QuickCheckVal {
    match normalisation {
        Normalisation::NFC => {
            if NFC_QC_M.contains(&code_point) {
                QuickCheckVal::Maybe
            } else if NFC_QC_N.contains(&code_point) {
                QuickCheckVal::No
            } else {
                QuickCheckVal::Yes
            }
        }
        Normalisation::NFD => {
            if NFD_QC_N.contains(&code_point) {
                QuickCheckVal::No
            } else {
                QuickCheckVal::Yes
            }
        }
        // Normalisation::NFKC => todo!(),
        // Normalisation::NFKD => todo!(),
    }
}

pub fn primary_composite(l: u32, c: u32) -> Option<u32> {
    PRIMARY_COMPOSITES.get(&[l, c]).map(|cp| *cp)
}

pub fn lowercase_mapping(code_point: u32) -> Option<u32> {
    LOWERCASE_MAPPINGS.get(&code_point).map(|cp| *cp)
}

pub fn uppercase_mapping(code_point: u32) -> Option<Vec<u32>> {
    UPPERCASE_MAPPINGS.get(&code_point).map(|cp| cp.clone())
}

pub fn cased(code_point: u32) -> bool {
    CASED.contains(&code_point)
}

pub fn case_ignorable(code_point: u32) -> bool {
    CASE_IGNORABLE.contains(&code_point)
}

pub fn case_folding(code_point: u32) -> Option<Vec<u32>> {
    FULL_CASE_FOLDING.get(&code_point).map(|cps| cps.clone())
}

// https://unicode.org/reports/tr29/#Grapheme_Cluster_Break_Property_Values
#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
pub enum GraphemeClusterBreak {
    CN,  // control char, separator
    CR,  // carriage return
    EB,  // e_base, obsolete & unused
    EBG, // e_base_gwz, obsolete & unused
    EM,  // e_modifier, obsolete & unused
    EX,  // extend
    GAZ, // glue after zwj, obsolete & unused
    L,   // Hangul Syllable Type L
    LF,  // line feed
    LV,  // Hangul Syllable Type LV
    LVT, // Hangul Syllable Type LVT
    PP,  // prepend
    RI,  // regional indicator
    SM,  // spacing mark
    T,   // Hangul Syllable Type T
    V,   // Hangul Syllable Type V
    XX,  // unknown
    ZWJ, // zero width joiner
}

pub fn grapheme_cluster_break(code_point: u32) -> GraphemeClusterBreak {
    GRAPHEME_CLUSTER_BREAK
        .get(&code_point)
        .map(|gcb| *gcb)
        .unwrap_or(GraphemeClusterBreak::XX)
}

pub fn extended_pictorial(code_point: u32) -> bool {
    EXTENDED_PICTORIAL.contains(&code_point)
}
