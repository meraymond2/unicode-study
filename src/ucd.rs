use std::collections::{HashMap, HashSet};
use lazy_static::lazy_static;

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
}

pub fn decomposition_mapping(code_point: u32) -> Option<Vec<u32>> {
    DECOMPOSITION_MAPPINGS.get(&code_point).map(|mapping| mapping.clone())
}

pub fn combining_class(code_point: u32) -> u8 {
    COMBINING_CLASSES.get(&code_point).map(|ccc| *ccc).unwrap_or(0)
}

pub fn is_starter(code_point: u32) -> bool {
    combining_class(code_point) == 0
}

pub enum QuickCheckVal {
    Yes,
    No,
    Maybe,
}

pub fn nfc_is_allowed(code_point: u32) -> QuickCheckVal {
    if NFC_QC_M.contains(&code_point) {
        QuickCheckVal::Maybe
    } else if NFC_QC_N.contains(&code_point) {
        QuickCheckVal::No
    } else {
        QuickCheckVal::Yes
    }
}


pub fn primary_composite(l: u32, c: u32) -> Option<u32> {
    PRIMARY_COMPOSITES.get(&[l, c]).map(|cp| *cp)
}
