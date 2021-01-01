/// This is the uncleaned result of XML deserialization.  You probably want the objects and methods
/// in the `geom::graph` module instead.

extern crate serde;

//use serde::{de::Error, Deserialize, Deserializer};
use serde::Deserialize;

use crate::Error as Error;

/// A `Mesh` is the top-level structure representing a GEOM object graph.
///
/// The mesh contains objects from various classes, called "geoms."  `Geom`s represent things like
/// disks, or disk partitions, or device nodes under `/dev` on FreeBSD systems.  They are related
/// by references called "consumers" and "providers."
#[derive(Debug, Deserialize, PartialEq)]
pub struct Mesh {
    #[serde(rename = "class", default)]
    pub classes: Vec<Class>,
}

/// `Class` contains all of the objects ("geoms") and associated relationships ("consumers" and
/// "providers") associated with the class.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Class {
    // Ideally, deserialize directly to u64.  However, neither of these works:
    //#[serde(with = "SerHex::<CompactPfx>")]
    //#[serde(borrow, deserialize_with = "from_hex")]
    pub id: String, // uintptr_t
    pub name: String,
    #[serde(rename = "geom", default)]
    pub geoms: Vec<Geom>,
    // libgeom(3) thinks Classes have config sections, but I don't see any.
}

/// A `Geom` is the essential object in a GEOM graph.
///
/// It can represent a disk (under the "DISK" class), or partition (under "PART"), or `/dev` device
/// node (under "DEV"), as well as several other classes.
///
/// A geom is related to other geoms in a directed graph.  `Consumer` edges indicate that this geom
/// depends on a lower-level (lower "`rank`") geom.  `Provider` edges indicate that this geom
/// exposes an object to a higher-level object.  For example, a PART geom might "consume" a DISK
/// geom ("ada0") and "provide" logical partition objects ("ada0p1", "ada0p2", etc.).
#[derive(Debug, Deserialize, PartialEq)]
pub struct Geom {
    pub id: String, // uintptr_t
    #[serde(rename = "class")]
    pub class_ref: ClassRef,
    pub name: String,
    pub rank: u64,
    pub config: Option<GeomConfig>,
    #[serde(rename = "consumer", default)]
    pub consumers: Vec<Consumer>,
    #[serde(rename = "provider", default)]
    pub providers: Vec<Provider>,
}

/// A `ClassRef` is just a logical pointer to a `Class`.
///
/// `ClassRef::ref_` references the same namespace as `Class::id`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ClassRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
}

/// A set of key-value metadata associated with a specific `Geom`.
///
/// The semantics and available values vary depending on the class.
#[derive(Debug, Deserialize, PartialEq)]
pub struct GeomConfig {
    // PART
    pub scheme: Option<String>,
    pub entries: Option<u64>,
    pub first: Option<u64>,
    pub last: Option<u64>,
    pub fwsectors: Option<u64>,
    pub fwheads: Option<u64>,
    pub state: Option<String>, // "OK"
    pub modified: Option<bool>,
}

/// A pointer from one geom to a `Provider` of a lower-level geom.
///
/// In the logical directed graph, it is an out-edge.
///
/// It is associated with the `Geom` with `id` equal to `geom_ref.ref_`, and points to the
/// `Provider` with `id` equal to `provider_ref.ref_`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Consumer {
    pub id: String, // uintptr_t
    #[serde(rename = "geom")]
    pub geom_ref: GeomRef,
    #[serde(rename = "provider")]
    pub provider_ref: ProviderRef,
    pub mode: String,
}

/// A pointer into a geom from the `Consumer` of a higher-level geom.
///
/// In the logical directed graph, it is an in-edge.
///
/// It is associated with the `Geom` with `id` equal to `geom_ref.ref_`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Provider {
    pub id: String, // uintptr_t
    #[serde(rename = "geom")]
    pub geom_ref: GeomRef,
    pub mode: String,
    pub name: String,
    pub mediasize: u64,
    pub sectorsize: u64,
    pub stripesize: u64,
    pub stripeoffset: u64,
    pub config: ProviderConfig,
}

// Ideally this would be some enum type based on the Class, but, ya know.  (serde(flatten) / enum
// interaction doesn't seem flawless at this time.)
/// A set of key-value metadata associated with a specific `Provider`.
///
/// The semantics and available values vary depending on the class.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ProviderConfig {
    // DISK
    pub fwheads: Option<u64>,
    pub fwsectors: Option<u64>,
    pub rotationrate: Option<u64>,
    pub ident: Option<String>,
    pub lunid: Option<String>,
    pub descr: Option<String>,
    // PART
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub index: Option<u64>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub offset: Option<u64>,
    pub length: Option<u64>,
    pub label: Option<String>,
    pub rawtype: Option<String>,
    pub rawuuid: Option<String>,
    pub efimedia: Option<String>,
    // LABEL
    // index, length, offset shared with PART above
    pub seclength: Option<u64>,
    pub secoffset: Option<u64>,
}

/// A `GeomRef` is just a logical pointer to a `Geom`.
///
/// `GeomRef::ref_` references the same namespace as `Geom::id`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct GeomRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
}

/// A `ProviderRef` is just a logical pointer to a `Provider`.
///
/// `ProviderRef::ref_` references the same namespace as `Provider::id`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ProviderRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
}

/// Parse a GEOM XML string configuration into a geom::raw::Mesh structure.
///
/// # Arguments
///
/// * `xml` - A string slice of the contents of the `kern.geom.confxml` `sysctl` node from a
///   FreeBSD system.
///
/// # Examples
///
/// ```
/// use freebsd_geom as geom;
///
/// let mesh = geom::raw::parse_xml(r#"<mesh> ... </mesh>"#).unwrap();
/// println!("The mesh has {} classes.", mesh.classes.len());
/// ```
pub fn parse_xml(xml: &str) -> Result<Mesh, Error> {
    return Ok(quick_xml::de::from_str::<Mesh>(xml)?);
}

/// Returns a structure representing the raw GEOM mesh on the running system.
///
/// # Examples
///
/// ```
/// use freebsd_geom as geom;
/// use std::collections::BTreeMap;
///
/// fn myfoo() -> Result<(), geom::Error> {
///     let mesh = geom::raw::get_mesh()?;
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
pub fn get_mesh() -> Result<Mesh, Error> {
    let xml = crate::get_confxml()?;
    return Ok(parse_xml(&xml)?);
}

#[cfg(test)]
mod tests {
    use crate::structs;

    #[test]
    fn xml_mesh_basic() {
        let xml = "<mesh></mesh>";
        quick_xml::de::from_str::<structs::Mesh>(xml).unwrap();
    }

    #[test]
    fn xml_class_basic() {
        let xml = "<class id=\"0xffffffff81234567\"><name>FD</name></class>";
        let cls = quick_xml::de::from_str::<structs::Class>(xml).unwrap();
        assert_eq!(cls.id, "0xffffffff81234567");
        assert_eq!(cls.name, "FD");
    }

    #[test]
    fn xml_references() {
        let xml = "<class ref=\"0x123\"/>";
        let cls = quick_xml::de::from_str::<structs::ClassRef>(xml).unwrap();
        assert_eq!(cls.ref_, "0x123");

        let xml = "<provider ref=\"0x123\"/>";
        let p = quick_xml::de::from_str::<structs::ProviderRef>(xml).unwrap();
        assert_eq!(p.ref_, "0x123");

        let xml = "<geom ref=\"0x123\"/>";
        let p = quick_xml::de::from_str::<structs::GeomRef>(xml).unwrap();
        assert_eq!(p.ref_, "0x123");
    }

    #[test]
    fn xml_geom_config() {
        let xml = r#"<config><scheme>GPT</scheme></config>"#;
        let p = quick_xml::de::from_str::<structs::GeomConfig>(xml).unwrap();
        assert_eq!(p.scheme.unwrap(), "GPT");
    }

    #[test]
    fn xml_consumer() {
        let xml = r#"<consumer id="0x123">
                        <geom ref="0x456"/>
                        <provider ref="0x789"/>
                        <mode>r0w0e0</mode>
                    </consumer>"#;
        let p = quick_xml::de::from_str::<structs::Consumer>(xml).unwrap();
        assert_eq!(p,
                   structs::Consumer {
                       id: "0x123".into(),
                       geom_ref: structs::GeomRef { ref_: "0x456".into() },
                       provider_ref: structs::ProviderRef { ref_: "0x789".into() },
                       mode: "r0w0e0".into(),
                   });

    }

    #[test]
    fn xml_provider_config() {
        // DISK class
        let xml = r#"<config>
                        <fwheads>1</fwheads>
                        <fwsectors>2</fwsectors>
                        <rotationrate>0</rotationrate>
                        <ident>S3Z</ident>
                        <lunid>00123abcd</lunid>
                        <descr>Samsung SSD</descr>
                    </config>"#;
        let p = quick_xml::de::from_str::<structs::ProviderConfig>(xml).unwrap();
        assert_eq!(p,
                   structs::ProviderConfig {
                       fwheads: Some(1),
                       fwsectors: Some(2),
                       rotationrate: Some(0),
                       ident: Some("S3Z".into()),
                       lunid: Some("00123abcd".into()),
                       descr: Some("Samsung SSD".into()),
                       // PART fields
                       start: None,
                       end: None,
                       index: None,
                       type_: None,
                       offset: None,
                       length: None,
                       label: None,
                       rawtype: None,
                       rawuuid: None,
                       efimedia: None,
                       // LABEL fields
                       seclength: None,
                       secoffset: None,
                   });
    }

    #[test]
    fn xml_provider() {
        let xml = r#"<provider id="0x123">
                        <geom ref="0x456"/>
                        <mode>r1w1e3</mode>
                        <name>ada0</name>
                        <mediasize>10</mediasize>
                        <sectorsize>2</sectorsize>
                        <stripesize>0</stripesize>
                        <stripeoffset>123</stripeoffset>
                        <config>
                            <fwheads>1</fwheads>
                            <fwsectors>2</fwsectors>
                            <rotationrate>0</rotationrate>
                            <ident>S3Z</ident>
                            <lunid>00123abcd</lunid>
                            <descr>Samsung SSD</descr>
                        </config>
                    </provider>"#;
        let p = quick_xml::de::from_str::<structs::Provider>(xml).unwrap();
        assert_eq!(p.id, "0x123");
        assert_eq!(p.geom_ref, structs::GeomRef { ref_: "0x456".into() });
        assert_eq!(p.mode, "r1w1e3");
        assert_eq!(p.name, "ada0");
        assert_eq!(p.mediasize, 10);
        assert_eq!(p.sectorsize, 2);
        assert_eq!(p.stripesize, 0);
        assert_eq!(p.stripeoffset, 123);
    }

    #[test]
    fn xml_geom_basic() {
        let xml = r#"<geom id="0x123">
                        <class ref="0x456"/>
                        <name>ada0</name>
                        <rank>1</rank>
                        <config>
                        </config>
                    </geom>"#;
        let p = quick_xml::de::from_str::<structs::Geom>(xml).unwrap();
        assert_eq!(p,
                   structs::Geom {
                       id: "0x123".into(),
                       class_ref: structs::ClassRef { ref_: "0x456".into() },
                       name: "ada0".into(),
                       rank: 1,
                       config: Some(structs::GeomConfig {
                           scheme: None,
                           entries: None,
                           first: None,
                           last: None,
                           fwsectors: None,
                           fwheads: None,
                           state: None,
                           modified: None,
                       }),
                       consumers: vec![],
                       providers: vec![],
                   });

    }

    #[test]
    fn xml_full_sample() {
        let xml = include_str!("test/fullsample.xml");
        let p = quick_xml::de::from_str::<structs::Mesh>(xml).unwrap();

        // Some arbitrarily chosen doc queries
        assert_eq!(p.classes[0].name, "FD");
        assert_eq!(p.classes[1].name, "RAID");

        assert_eq!(p.classes[2].name, "DISK");
        assert_eq!(p.classes[2].id,
                   p.classes[2].geoms[0].class_ref.ref_);
        assert_eq!(p.classes[2].geoms[0].id,
                   p.classes[2].geoms[0].providers[0].geom_ref.ref_);
        assert_eq!(p.classes[2].geoms[0].providers[0].mediasize, 1000204886016);
        assert_eq!(p.classes[2].geoms[0].providers[0].config.lunid.as_ref().unwrap(),
                   "YYYYYYYYYYYYYYYY");
        assert_eq!(p.classes[2].geoms[1].name, "nvd1");

        assert_eq!(p.classes[3].name, "DEV");
        assert_eq!(p.classes[3].id,
                   p.classes[3].geoms[0].class_ref.ref_);
        assert_eq!(p.classes[3].geoms[1].name, "ada0p1");

        // DEV consumer -> PART provider
        assert_eq!(p.classes[3].geoms[1].consumers[0].provider_ref.ref_,
                   p.classes[4].geoms[0].providers[0].id);

        assert_eq!(p.classes[4].name, "PART");
        assert_eq!(p.classes[5].name, "LABEL");
        assert_eq!(p.classes[6].name, "VFS");
        assert_eq!(p.classes[7].name, "SWAP");
        assert_eq!(p.classes[8].name, "Flashmap");
        assert_eq!(p.classes[9].name, "MD");
    }
}
