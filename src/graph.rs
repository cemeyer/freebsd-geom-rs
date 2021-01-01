use std::collections::{BTreeMap,BTreeSet};
use std::str::FromStr;
use strum_macros::{AsRefStr,EnumIter,EnumString};
use crate::raw;

/// Some internal graph invariant was violated.
pub struct GraphError;

pub struct Geom {
    pub class: GeomClass,
    pub name: String,
    pub rank: u64,
    pub config: Option<Box<PartConfig>>,
}

/// Classes a `Geom` might belong to
#[derive(Copy,Clone,Eq,PartialEq,AsRefStr,EnumIter,EnumString)]
pub enum GeomClass {
    FD,
    RAID,
    DISK,
    DEV,
    PART,
    LABEL,
    VFS,
    SWAP,
    Flashmap,
    MD,
}

/// PART geom partition schemes
#[derive(AsRefStr,EnumIter,EnumString)]
pub enum PartScheme {
    APM,
    BSD,
    BSD64,
    EBR,
    GPT,
    LDM,
    MBR,
    VTOC8,
}

#[derive(AsRefStr,EnumIter,EnumString)]
pub enum PartState {
    CORRUPT,
    OK,
}

/// Config metadata associated with PART geoms.
pub struct PartConfig {
    scheme: PartScheme,
    entries: u64,
    first: u64,
    last: u64,
    fwsectors: u64,
    fwheads: u64,
    state: PartState,
    modified: bool,
}

/// GEOM internal access refcounts.
pub struct Mode {
    read: u16,
    write: u16,
    exclusive: u16,
}

impl std::str::FromStr for Mode {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Mode, Self::Err> {
        let (r, w, e) = scan_fmt!(s, "r{d}w{d}e{d}", u16, u16, u16)
            .map_err(|e| crate::Error::Scan(e))?;
        return Ok(Mode {
            read: r,
            write: w,
            exclusive: e,
        });
    }
}

// Keyed off the type of the geom associated with the provider.
#[derive(AsRefStr,EnumIter,EnumString)]
pub enum ProviderConfig {
    DISK {
        fwheads: u64,
        fwsectors: u64,
        rotationrate: u64,
        ident: String,
        lunid: String,
        descr: String,
    },
    PART {
        start: u64,
        end: u64,
        index: u64,
        type_: String, // theoretically, a big enum ("freebsd-ufs")
        offset: u64,
        length: u64,
        // These ones are optional / vary by partition scheme.  These are the GPT ones:
        // attrib: xxx,
        label: Option<String>,
        rawtype: Option<String>,
        rawuuid: Option<String>,
        efimedia: Option<String>,
    },
    // LABEL, Flashmap both create "slices".
    #[strum(serialize="LABEL",serialize="Flashmap")]
    SLICE {
        index: u64,
        offset: u64,
        length: u64,
        seclength: u64,
        secoffset: u64,
    },
}

impl ProviderConfig {
    fn disk_from_raw(p: &raw::Provider) -> Result<Box<ProviderConfig>, crate::Error> {
        let raw = &p.config;
        Ok(Box::new(Self::DISK {
            fwheads: raw.fwheads.ok_or(crate::Error::Graph(GraphError))?,
            fwsectors: raw.fwsectors.ok_or(crate::Error::Graph(GraphError))?,
            rotationrate: raw.rotationrate.ok_or(crate::Error::Graph(GraphError))?,
            ident: raw.ident.as_ref().ok_or(crate::Error::Graph(GraphError))?.to_owned(),
            lunid: raw.lunid.as_ref().ok_or(crate::Error::Graph(GraphError))?.to_owned(),
            descr: raw.descr.as_ref().ok_or(crate::Error::Graph(GraphError))?.to_owned(),
        }))
    }

    fn part_from_raw(p: &raw::Provider) -> Result<Box<ProviderConfig>, crate::Error> {
        let raw = &p.config;
        Ok(Box::new(Self::PART {
            start: raw.start.ok_or(crate::Error::Graph(GraphError))?,
            end: raw.end.ok_or(crate::Error::Graph(GraphError))?,
            index: raw.index.ok_or(crate::Error::Graph(GraphError))?,
            type_: raw.type_.as_ref().ok_or(crate::Error::Graph(GraphError))?.to_owned(),
            offset: raw.offset.ok_or(crate::Error::Graph(GraphError))?,
            length: raw.length.ok_or(crate::Error::Graph(GraphError))?,

            label:       raw.label.as_ref().map(|v| v.to_owned()),
            rawtype:   raw.rawtype.as_ref().map(|v| v.to_owned()),
            rawuuid:   raw.rawuuid.as_ref().map(|v| v.to_owned()),
            efimedia: raw.efimedia.as_ref().map(|v| v.to_owned()),
        }))
    }

    fn slice_from_raw(p: &raw::Provider) -> Result<Box<ProviderConfig>, crate::Error> {
        let raw = &p.config;
        Ok(Box::new(Self::SLICE {
            index: raw.index.ok_or(crate::Error::Graph(GraphError))?,
            offset: raw.offset.ok_or(crate::Error::Graph(GraphError))?,
            length: raw.length.ok_or(crate::Error::Graph(GraphError))?,
            seclength: raw.seclength.ok_or(crate::Error::Graph(GraphError))?,
            secoffset: raw.secoffset.ok_or(crate::Error::Graph(GraphError))?,
        }))
    }
}

/// Represents a Consumer-Provider pair.
pub struct Edge {
    pub name: String,
    pub mode: Mode,
    pub mediasize: u64,
    pub sectorsize: u64,
    pub stripesize: u64,
    pub stripeoffset: u64,
    pub config: Option<Box<ProviderConfig>>,
}

pub type NodeId = u64;
pub type EdgeId = (u64, u64);

pub struct Graph {
    pub nodes: BTreeMap<NodeId, Geom>,
    pub edges: BTreeMap<EdgeId, Edge>,
    // adjacency lists:
    pub outedges: BTreeMap<NodeId, Vec<EdgeId>>,
    pub inedges: BTreeMap<NodeId, Vec<EdgeId>>,
}

impl Graph {
    fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            outedges: BTreeMap::new(),
            inedges: BTreeMap::new(),
        }
    }
}

fn scan_ptr(s: &str) -> Result<u64, crate::Error> {
    let p = scan_fmt!(s, "{x}", [hex u64])
        .map_err(|e| crate::Error::Scan(e))?;
    return Ok(p)
}

// XXX double check that all providers are attached to a consumer, but I think they are via DEV.
pub fn decode_graph(mesh: &raw::Mesh) -> Result<Graph, crate::Error> {
    let mut result = Graph::new();

    // First pass: create nodes; temp: collect consumers, producers, and their pairs.
    let mut cons: BTreeMap<u64, &raw::Consumer> = BTreeMap::new();
    let mut provs: BTreeMap<u64, &raw::Provider> = BTreeMap::new();
    let mut conprods: BTreeSet<EdgeId> = BTreeSet::new();

    for class in &mesh.classes {
        let classkind = GeomClass::from_str(&class.name)
            .map_err(|e| crate::Error::Parse(e))?;

        for geom in &class.geoms {
            let geom_id = scan_ptr(&geom.id)?;
            let mut config = None;
            if classkind == GeomClass::PART {
                let rawconfig = &geom.config.as_ref()
                    .ok_or(crate::Error::Graph(GraphError))?;
                let partscheme = PartScheme::from_str(&rawconfig.scheme
                    .as_ref()
                    .ok_or(crate::Error::Graph(GraphError))?)
                    .map_err(|e| crate::Error::Parse(e))?;
                let partstate = PartState::from_str(&rawconfig.state
                    .as_ref()
                    .ok_or(crate::Error::Graph(GraphError))?)
                    .map_err(|e| crate::Error::Parse(e))?;

                config = Some(Box::new(PartConfig {
                    scheme: partscheme,
                    state: partstate,
                    entries:
                        rawconfig.entries.ok_or(crate::Error::Graph(GraphError))?,
                    first:
                        rawconfig.first.ok_or(crate::Error::Graph(GraphError))?,
                    last:
                        rawconfig.last.ok_or(crate::Error::Graph(GraphError))?,
                    fwsectors:
                        rawconfig.fwsectors.ok_or(crate::Error::Graph(GraphError))?,
                    fwheads:
                        rawconfig.fwheads.ok_or(crate::Error::Graph(GraphError))?,
                    modified:
                        rawconfig.modified.ok_or(crate::Error::Graph(GraphError))?,
                }));
            }
            result.nodes.insert(geom_id, Geom {
                class: classkind,
                name: geom.name.to_owned(),
                rank: geom.rank,
                config: config,
            });

            for c in &geom.consumers {
                let cons_id = scan_ptr(&c.id)?;
                let prov_id = scan_ptr(&c.provider_ref.ref_)?;

                cons.insert(cons_id, &c);
                conprods.insert((cons_id, prov_id));
            }
            for p in &geom.providers {
                let prov_id = scan_ptr(&p.id)?;
                provs.insert(prov_id, &p);
            }
        }
    }

    // Second pass: create Con-Prov Edges; fill inedges, outedges.
    for (cid, pid) in &conprods {
        let rawcons = cons.get(&cid).ok_or(crate::Error::Graph(GraphError))?;
        let rawprov = provs.get(&pid).ok_or(crate::Error::Graph(GraphError))?;
        if &rawcons.mode != &rawprov.mode {
            return Err(crate::Error::Graph(GraphError));
        }

        // Geom associated with the provider in this pair.
        let provgeom_id = scan_ptr(&rawprov.geom_ref.ref_)?;
        let provgeom = result.nodes.get(&provgeom_id).ok_or(crate::Error::Graph(GraphError))?;

        let edge = Edge {
            name: rawprov.name.to_owned(),
            mode: Mode::from_str(&rawprov.mode)?,
            mediasize: rawprov.mediasize,
            sectorsize: rawprov.sectorsize,
            stripesize: rawprov.stripesize,
            stripeoffset: rawprov.stripeoffset,
            config: match provgeom.class {
                GeomClass::DISK => Some(ProviderConfig::disk_from_raw(rawprov)?),
                GeomClass::PART => Some(ProviderConfig::part_from_raw(rawprov)?),
                GeomClass::LABEL |
                GeomClass::Flashmap => Some(ProviderConfig::slice_from_raw(rawprov)?),
                _ => None,
            }
        };

        let edge_id = (*cid, *pid);
        result.edges.insert(edge_id, edge);

        let invec = result.inedges.entry(provgeom_id).or_insert(Vec::new());
        (*invec).push(edge_id);

        let consgeom_id = scan_ptr(&rawcons.geom_ref.ref_)?;
        let outvec = result.outedges.entry(consgeom_id).or_insert(Vec::new());
        (*outvec).push(edge_id);
    }

    return Ok(result);
}

#[cfg(test)]
mod tests {
    use crate::{raw, graph};

    #[test]
    fn large_sample_decode() {
        let xml = include_str!("test/fullsample.xml");
        let rawmesh = raw::parse_xml(&xml).unwrap();
        let g = graph::decode_graph(&rawmesh);
    }
}
