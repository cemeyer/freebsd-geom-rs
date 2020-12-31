#![allow(dead_code)]

mod structs;

extern crate sysctl;

use sysctl::Sysctl;

#[cfg(target_os = "freebsd")]
fn get_geom_confxml() -> Result<String, sysctl::SysctlError> {
    const CTLNAME: &str = "kern.geom.confxml";

    let ctl = sysctl::Ctl::new(CTLNAME)?;
    return ctl.value_string();
}

#[cfg(all(test, target_os = "freebsd"))]
mod tests_freebsd {
    use crate::*;

    #[test]
    fn getsysctlstr() {
        let s = get_geom_confxml().expect("get_geom_confxml");
        assert_ne!(s, "", "sysctl output is non-empty");
    }
}
