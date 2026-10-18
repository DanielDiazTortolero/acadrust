//! Every paper-space layout's BLOCK begin marker keeps its numbered name in DWG.
//!
//! AutoCAD stores `*Paper_Space` on each paper-space BLOCK_HEADER, but the BLOCK markers of the non-active
//! layouts carry `*Paper_Space0`, `*Paper_Space1`, ...; that is what tells the records apart. A DWG whose
//! markers are all named `*Paper_Space` opens only through recovery in AutoCAD ("Damaged File") and other
//! readers reject it ("the entry *Paper_Space already exists"). Documents read from DXF or built in code
//! keep no BLOCK marker entities, so the writer must give the fallback marker the record's own name.
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};
use std::io::Cursor;

/// BLOCK marker names as stored in the DWG (seen before the reader reconciles them with their records).
fn paper_space_marker_names(bytes: Vec<u8>) -> Vec<String> {
    let mut names = Vec::new();
    DwgReader::from_stream(Cursor::new(bytes))
        .read_visiting(|_, entity| {
            if let EntityType::Block(block) = &entity {
                names.push(block.name.clone());
            }
            Some(entity)
        })
        .unwrap();
    let mut names: Vec<String> = names
        .into_iter()
        .filter(|n| n.to_ascii_uppercase().starts_with("*PAPER_SPACE"))
        .collect();
    names.sort();
    names
}

fn two_extra_layouts() -> CadDocument {
    let mut doc = CadDocument::new();
    doc.add_layout("E-101").unwrap();
    doc.add_layout("E-102").unwrap();
    doc
}

#[test]
fn layouts_built_in_code_write_numbered_paper_space_markers() {
    let doc = two_extra_layouts();
    let names = paper_space_marker_names(DwgWriter::write_to_vec(&doc).unwrap());
    assert_eq!(names, ["*Paper_Space", "*Paper_Space0", "*Paper_Space1"]);
}

#[test]
fn layouts_read_from_dxf_write_numbered_paper_space_markers() {
    let dxf = DxfWriter::new(&two_extra_layouts()).write_to_vec().unwrap();
    let doc = DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap();
    let names = paper_space_marker_names(DwgWriter::write_to_vec(&doc).unwrap());
    assert_eq!(names, ["*Paper_Space", "*Paper_Space0", "*Paper_Space1"]);
}
