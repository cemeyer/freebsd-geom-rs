#![allow(dead_code)]

#[macro_use]
extern crate scan_fmt;
extern crate sysctl;

use sysctl::Sysctl;

#[cfg(target_os = "freebsd")]
fn get_confxml() -> Result<String, Error> {
    const CTLNAME: &str = "kern.geom.confxml";

    let ctl = sysctl::Ctl::new(CTLNAME)?;
    return Ok(ctl.value_string()?);
}

/// Returns a structure representing the GEOM graph on the running system.
///
/// # Examples
///
/// ```
/// use freebsd_geom as geom;
///
/// fn myfoo() -> Result<(), geom::Error> {
///     let graph = geom::get_graph()?;
///     Ok(())
/// }
/// ```
#[cfg(target_os = "freebsd")]
pub fn get_graph() -> Result<Graph, Error> {
    let raw_mesh = raw::get_mesh()?;
    return graph::decode_graph(&raw_mesh);
}

#[cfg(all(test, target_os = "freebsd"))]
mod tests_freebsd {
    use crate::*;

    #[test]
    #[ignore = "not reproducible"]
    fn getsysctlstr() {
        let s = get_confxml().unwrap();
        assert_ne!(s, "", "sysctl output is non-empty");
    }
}

// reexport
pub mod error;
mod graph;
pub mod structs;

pub use error::Error;
pub use graph::{
    Edge, EdgeId, EdgeMetadata, Geom, GeomClass, Graph, Mode, NodeId, PartMetadata, PartScheme,
    PartState,
};
pub use structs as raw;
