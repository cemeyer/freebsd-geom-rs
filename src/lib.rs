#![allow(dead_code)]

// reexport
pub mod structs;
pub use structs as raw;

extern crate sysctl;

use sysctl::Sysctl;

/// Wrapped error sources for geom::get_mesh().
pub enum Error {
    Sysctl(sysctl::SysctlError),
    Decode(quick_xml::DeError),
}

#[cfg(target_os = "freebsd")]
fn get_confxml() -> Result<String, sysctl::SysctlError> {
    const CTLNAME: &str = "kern.geom.confxml";

    let ctl = sysctl::Ctl::new(CTLNAME)?;
    return ctl.value_string();
}

/// Returns a structure representing the GEOM mesh on the running system.
///
/// # Examples
///
/// ```
/// use freebsd_geom as geom;
/// use std::collections::BTreeMap;
///
/// fn myfoo() -> Result<(), geom::Error> {
///     let mesh = geom::get_mesh()?;
///
///     let mut count = BTreeMap::new();
///     for g_class in &mesh.classes {
///         count.insert(&g_class.name, g_class.geoms.len());
///     }
///     for (class_name, count) in &count {
///         println!("class {}: {} geoms", class_name, count);
///     }
///     Ok(())
/// }
/// ```
#[cfg(target_os = "freebsd")]
pub fn get_mesh() -> Result<raw::Mesh, Error> {
    let xml = get_confxml().map_err(|e| Error::Sysctl(e))?;
    return raw::parse_xml(&xml).map_err(|e| Error::Decode(e));
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
