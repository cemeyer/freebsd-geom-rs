#![allow(dead_code)]

#[macro_use] extern crate scan_fmt;
extern crate sysctl;

use sysctl::Sysctl;

/// Wrapped error sources for the geom crate.
pub enum Error {
    Sysctl(sysctl::SysctlError),
    Decode(quick_xml::DeError),
    Parse(strum::ParseError),
    Scan(scan_fmt::parse::ScanError),
    Graph(graph::GraphError),
}

#[cfg(target_os = "freebsd")]
fn get_confxml() -> Result<String, sysctl::SysctlError> {
    const CTLNAME: &str = "kern.geom.confxml";

    let ctl = sysctl::Ctl::new(CTLNAME)?;
    return ctl.value_string();
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
pub mod structs;
pub use structs as raw;
mod graph;
pub use graph::Graph;
