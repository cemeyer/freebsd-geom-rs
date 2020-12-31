extern crate serde;

//use serde::{de::Error, Deserialize, Deserializer};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Mesh {
    #[serde(rename = "class", default)]
    pub classes: Vec<Class>,
}

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

#[derive(Debug, Deserialize, PartialEq)]
pub struct ClassRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
}

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

#[derive(Debug, Deserialize, PartialEq)]
pub struct Consumer {
    pub id: String, // uintptr_t
    #[serde(rename = "geom")]
    pub geom_ref: GeomRef,
    #[serde(rename = "provider")]
    pub provider_ref: ProviderRef,
    pub mode: String,
}

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

#[derive(Debug, Deserialize, PartialEq)]
pub struct GeomRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ProviderRef {
    #[serde(rename = "ref")]
    pub ref_: String, // uintptr_t
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
