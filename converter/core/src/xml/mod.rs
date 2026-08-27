//! A minimal, namespace-aware XML tree plus a streaming reader and a
//! pretty-printing writer.
//!
//! Names are kept *expanded* — `(namespace uri, local name)` — never as raw
//! `prefix:local` strings. Prefixes are an encoding detail: the reader resolves
//! them away and the writer re-invents them from the output profile. That is
//! what makes a namespace bump (`citygml/2.0` -> `citygml/3.0`) a data change
//! rather than a string-substitution hazard.

mod node;
mod read;
mod write;

pub use node::{Attribute, Element, Name, Node};
pub use read::{Chunk, Reader, read_to_string};
pub use write::{Indent, PrefixMap, Writer};

/// Namespace URIs that the converter refers to by name.
pub mod ns {
    pub const GML_31: &str = "http://www.opengis.net/gml";
    pub const GML_32: &str = "http://www.opengis.net/gml/3.2";
    pub const XLINK: &str = "http://www.w3.org/1999/xlink";
    pub const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

    pub const CITYGML_2: &str = "http://www.opengis.net/citygml/2.0";
    pub const BUILDING_2: &str = "http://www.opengis.net/citygml/building/2.0";

    pub const CITYGML_3: &str = "http://www.opengis.net/citygml/3.0";
    pub const BUILDING_3: &str = "http://www.opengis.net/citygml/building/3.0";
    pub const CONSTRUCTION_3: &str = "http://www.opengis.net/citygml/construction/3.0";
}
