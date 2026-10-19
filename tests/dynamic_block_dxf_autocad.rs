//! Dynamic-block objects written to DXF as AutoCAD writes them (AutoCAD's own DXF of ACadSharp's
//! dynamic-block samples, MIT). DWG TrueView discarded the whole DXF over the lookup parameter's
//! and lookup grip's subclass markers (AcDbBlockLookUpParameter / AcDbBlockLookUpGrip in the
//! record; CLASSES keeps the C++ names) and over an alignment grip's orientation in 140/150/160
//! (AutoCAD: 140/141/142). AutoCAD also writes the lookup parameter's 303, 304 before its 94, and
//! the move, stretch and polar stretch actions' connections as id/name pairs.

use acadrust::{
    objects::{DynamicBlockData, ObjectType},
    CadDocument, DwgReader, DxfReader, DxfWriter,
};
use std::io::Cursor;

fn read(bytes: &[u8]) -> CadDocument {
    DwgReader::from_stream(Cursor::new(bytes)).read().unwrap()
}

fn dxf_text(doc: &CadDocument) -> String {
    String::from_utf8_lossy(&DxfWriter::new(doc).write_to_vec().unwrap()).into_owned()
}

fn read_dxf(text: &str) -> CadDocument {
    DxfReader::from_reader(Cursor::new(text.as_bytes().to_vec()))
        .unwrap()
        .read()
        .unwrap()
}

/// The (code, value) pairs of every `kind` record in a DXF text.
fn records(text: &str, kind: &str) -> Vec<Vec<(String, String)>> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = Vec::new();
    let mut current: Option<Vec<(String, String)>> = None;
    for pair in lines.chunks(2) {
        let [code, value] = pair else { break };
        if *code == "0" {
            if let Some(record) = current.take() {
                out.push(record);
            }
            if *value == kind {
                current = Some(Vec::new());
            }
        } else if let Some(record) = current.as_mut() {
            record.push((code.to_string(), value.to_string()));
        }
    }
    out.extend(current);
    out
}

/// The last subclass marker of a record and the group codes after it (xdata left out).
fn last_subclass(record: &[(String, String)]) -> (String, Vec<String>) {
    let at = record.iter().rposition(|(c, _)| c == "100").unwrap();
    let codes = record[at + 1..]
        .iter()
        .take_while(|(c, _)| c != "1001")
        .map(|(c, _)| c.clone())
        .collect();
    (record[at].1.clone(), codes)
}

fn objects<'a>(
    doc: &'a CadDocument,
    pick: impl Fn(&DynamicBlockData) -> bool + 'a,
) -> Vec<&'a DynamicBlockData> {
    let mut found: Vec<_> = doc
        .objects
        .iter()
        .filter_map(|(handle, object)| match object {
            ObjectType::DynamicBlock(object) if pick(&object.data) => Some((handle.value(), &object.data)),
            _ => None,
        })
        .collect();
    found.sort_by_key(|(handle, _)| *handle);
    found.into_iter().map(|(_, data)| data).collect()
}

#[test]
fn lookup_parameter_and_grip_use_autocads_markers() {
    let doc = read(include_bytes!("fixtures/dynamic-blocks/lookup.dwg"));
    let text = dxf_text(&doc);
    let parameters = records(&text, "BLOCKLOOKUPPARAMETER");
    assert!(!parameters.is_empty());
    for record in &parameters {
        let (marker, codes) = last_subclass(record);
        assert_eq!(marker, "AcDbBlockLookUpParameter");
        assert_eq!(codes, ["303", "304", "94"]);
    }
    let grips = records(&text, "BLOCKLOOKUPGRIP");
    assert!(!grips.is_empty());
    for record in &grips {
        assert_eq!(last_subclass(record).0, "AcDbBlockLookUpGrip");
    }
    // CLASSES keeps the C++ class names, as AutoCAD's DXF does.
    let class = records(&text, "CLASS")
        .into_iter()
        .find(|r| r.iter().any(|(c, v)| c == "1" && v == "BLOCKLOOKUPPARAMETER"))
        .unwrap();
    assert!(class.iter().any(|(c, v)| c == "2" && v == "AcDbBlockLookupParameter"));
}

#[test]
fn lookup_parameter_reads_back_from_both_spellings() {
    let doc = read(include_bytes!("fixtures/dynamic-blocks/lookup.dwg"));
    let is_lookup = |d: &DynamicBlockData| matches!(d, DynamicBlockData::LookupParameter(_));
    let original = objects(&doc, is_lookup);
    assert!(!original.is_empty());
    let text = dxf_text(&doc);
    // AutoCAD's spelling, and the C++ class name earlier acadrust output used.
    for text in [text.clone(), text.replace("AcDbBlockLookUpParameter", "AcDbBlockLookupParameter")] {
        let back = read_dxf(&text);
        let DynamicBlockData::LookupParameter(first) = objects(&back, is_lookup)[0] else {
            panic!()
        };
        let DynamicBlockData::LookupParameter(expected) = original[0] else {
            panic!()
        };
        assert_eq!(first.index, expected.index);
        assert_eq!(first.lookup_name, expected.lookup_name);
        assert_eq!(first.lookup_description, expected.lookup_description);
        assert!(!expected.lookup_name.is_empty());
    }
}

#[test]
fn alignment_grip_orientation_is_140_141_142() {
    let doc = read(include_bytes!("fixtures/dynamic-blocks/alignment.dwg"));
    let text = dxf_text(&doc);
    let grips = records(&text, "BLOCKALIGNMENTGRIP");
    assert!(!grips.is_empty());
    for record in &grips {
        let (marker, codes) = last_subclass(record);
        assert_eq!(marker, "AcDbBlockAlignmentGrip");
        assert_eq!(codes, ["140", "141", "142"]);
    }
    let is_grip = |d: &DynamicBlockData| matches!(d, DynamicBlockData::AlignmentGrip(_));
    let (before, back) = (objects(&doc, is_grip), read_dxf(&text));
    let after = objects(&back, is_grip);
    let (DynamicBlockData::AlignmentGrip(a), DynamicBlockData::AlignmentGrip(b)) = (before[0], after[0]) else {
        panic!()
    };
    assert!(a.orientation.x != 0.0 || a.orientation.y != 0.0);
    assert_eq!(a.orientation, b.orientation);
}

#[test]
fn move_stretch_and_polar_stretch_connections_are_id_name_pairs() {
    let lookup = dxf_text(&read(include_bytes!("fixtures/dynamic-blocks/lookup.dwg")));
    let point = dxf_text(&read(include_bytes!("fixtures/dynamic-blocks/point.dwg")));
    let polar = dxf_text(&read(include_bytes!("fixtures/dynamic-blocks/polar.dwg")));
    let pairs = |n: usize| -> Vec<String> {
        (0..n).flat_map(|i| [(92 + i).to_string(), (301 + i).to_string()]).collect()
    };
    for (text, kind, n) in [
        (&lookup, "BLOCKMOVEACTION", 2),
        (&point, "BLOCKMOVEACTION", 2),
        (&lookup, "BLOCKSTRETCHACTION", 2),
        (&polar, "BLOCKPOLARSTRETCHACTION", 6),
    ] {
        let found = records(text, kind);
        assert!(!found.is_empty(), "{kind}");
        for record in &found {
            let (_, codes) = last_subclass(record);
            assert_eq!(codes[..2 * n], pairs(n)[..], "{kind}");
        }
    }
    // The other actions keep AutoCAD's grouped form (ids, then names).
    let xy = dxf_text(&read(include_bytes!("fixtures/dynamic-blocks/xy.dwg")));
    for record in records(&xy, "BLOCKARRAYACTION") {
        let (_, codes) = last_subclass(&record);
        assert_eq!(codes[..8], ["92", "93", "94", "95", "301", "302", "303", "304"]);
    }
}
