/// The GEOM subsystem of FreeBSD is an abstraction of storage topology inside the kernel.
///
/// In math jargon, it is a "forest" of disconnected trees.  The root(s) of these trees are
/// individual `Geom` objects of class `GeomClass::DISK` or similar (e.g., `MD` — Memory Disk).
///
/// The leaves of the trees are `Geom` objects of type `GeomClass::DEV`, which are responsible for
/// constructing the virtual files present in `/dev`.

use std::collections::{BTreeMap,BTreeSet};
use std::str::FromStr;
use strum_macros::{AsRefStr,EnumIter,EnumString};
use crate::{Error, raw};

/// A `Geom` is the essential object in a GEOM graph.
///
/// It has a `name` and "`rank`" (a computed depth of the tree containing this geom).  It can
/// represent a disk (`GeomClass::DISK`), or partition (`GeomClass::PART`), or `/dev` device node
/// (`GeomClass::DEV`), as well as several other classes.
///
/// A geom *may* have some associated `metadata` (e.g., `PART` geoms).
///
/// A geom is related to other geoms in a tree.  In this library, we call edges from child to
/// parent geoms "outedges" and edges from parent geoms to child geoms "inedges".  In other GEOM
/// documentation they are called "consumers" and "providers," respectively.
#[derive(Debug)]
pub struct Geom {
    pub class: GeomClass,
    /// The `Geom`'s name, such as "ada0".  Caveat: geom names are not unique.
    pub name: String,
    /// The height of this `Geom` in its tree.  For example, a `Geom` at the root of a tree will
    /// have `rank` equal to `1`.
    pub rank: u64,
    /// If this `Geom` is `GeomClass::PART`, some additional metadata.
    pub metadata: Option<Box<PartMetadata>>,
}

/// The class of a `Geom`.
#[derive(Copy,Clone,Debug,Eq,PartialEq,AsRefStr,EnumIter,EnumString)]
pub enum GeomClass {
    /// Floppy Disk
    FD,
    RAID,
    /// Typical PC storage devices: SATA, NVMe, IDE
    DISK,
    /// Virtual "character device" in `/dev`
    DEV,
    /// Represents a partition table, such as GPT or MBR.
    PART,
    /// Represents aliases for other `Geom`s.  For example, disk serial number, GPT partition
    /// labels, or filesystem-internal labels (UFS, etc).
    LABEL,
    VFS,
    SWAP,
    Flashmap,
    /// A Memory Disk (virtual device)
    MD,
}

/// Specific partition schemes for `GeomClass::PART` geom `PartMetadata`.
#[derive(AsRefStr,Debug,EnumIter,EnumString)]
pub enum PartScheme {
    /// Apple Partition Map (historical)
    APM,
    /// FreeBSD disklabels (historical)
    BSD,
    /// DragonflyBSD disklabels (circa 2014)
    BSD64,
    /// Extended Boot Record (a historical scheme for working around MBR's limit of four
    /// partitions)
    EBR,
    /// GUID Partition Table (most common scheme today)
    GPT,
    /// Logical Disk Manager (historical, Windows 2000; deprecated in Windows 8)
    LDM,
    /// Master Boot Record (historical; hard limit of four partitions)
    MBR,
    /// Volume Table of Contents (historical: SPARC-only)
    VTOC8,
}

/// The `PartState::CORRUPT` state on a `GeomClass::PART` `Geom` indicates any of several possible
/// issues with metadata on the *parent* `Geom`.
///
/// `PartState::OK` indicates the absence of any of these issues.
///
/// Corruption issues can include:
/// * GPT scheme: Either the primary or secondary GPT header is corrupt.  If one is intact, the
///   other can be recovered.
/// * EBR scheme: An internal inconsistency exists in EBR's metadata.
/// * Any scheme: There is some internal inconsistency, such as overlapping partitions.
#[derive(AsRefStr,Debug,EnumIter,EnumString)]
pub enum PartState {
    CORRUPT,
    OK,
}

/// Metadata associated with `GeomClass::PART` `Geom`s.
#[derive(Debug)]
pub struct PartMetadata {
    /// The partitioning scheme
    scheme: PartScheme,
    /// The number of partitions in this table
    entries: u64,
    /// First allocatable LBA
    first: u64,
    /// Last alloctable LBA
    last: u64,
    /// Historical: "S" in "CHS geometry"
    fwsectors: u64,
    /// Historical: "H" in "CHS geometry"
    fwheads: u64,
    /// Internal consistency of the partition table
    state: PartState,
    /// If the partition table has been modified and not yet written
    modified: bool,
}

/// GEOM internal access reference counts
#[derive(Debug)]
pub struct Mode {
    read: u16,
    write: u16,
    exclusive: u16,
}

impl std::str::FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Mode, Self::Err> {
        let (r, w, e) = scan_fmt!(s, "r{d}w{d}e{d}", u16, u16, u16)?;
        return Ok(Mode {
            read: r,
            write: w,
            exclusive: e,
        });
    }
}

// Keyed off the type of the geom associated with the provider.
/// Metadata associated with an `Edge`.
///
/// The enum variant depends on the `GeomClass` of the `Geom` associated with the "provider"
/// represented by this `Edge`.
#[derive(AsRefStr,Debug,EnumIter,EnumString)]
pub enum EdgeMetadata {
    /// `EdgeMetadata::DISK` is metadata associated with the `Edge` between a `GeomClass::DISK`
    /// `Geom` and some lower `Geom` in the tree.
    DISK {
        /// Historical: "H" in "CHS geometry"
        fwheads: u64,
        /// Historical: "S" in "CHS geometry"
        fwsectors: u64,
        /// Zero indicates solid-state drives; non-zero represents spinning drives.
        rotationrate: u64,
        /// Serial number, or some other identifier
        ident: String,
        /// LUN identifier.  Logical Unit Numbers come from SCSI, but are synthesized for other
        /// non-SCSI devices in FreeBSD's CAM.
        lunid: String,
        /// A description.  For example, disk make and model.
        descr: String,
    },
    /// `EdgeMetadata::PART` is metadata associated with the `Edge` between a `GeomClass::PART` and
    /// some lower `Geom` in the tree.
    ///
    /// These edges exist for each partition *entry*, whereas there is only one `PART` `Geom` for
    /// the entire partition *table*.
    PART {
        /// First LBA of partition entry
        start: u64,
        /// Last LBA of partition entry
        end: u64,
        /// Index of partition entry in partition table
        index: u64,
        /// A canonical FreeBSD GEOM alias for the filesystem type metadata associated with this
        /// partition entry.  E.g., both MBR `0xef` and GPT "C12A7328-F81F-11D2-BA4B-00A0C93EC93B"
        /// are mapped to the same alias: `G_PART_ALIAS_EFI`, or `"efi"`.
        ///
        /// The complete list may be found in `sys/geom/part/g_part.c` in the `g_part_alias_list`
        /// table.
        type_: String, // theoretically, a big enum, but we'd have to extract it from g_part.c
        /// The byte offset of the start of the partition entry
        offset: u64,
        /// The length of the partition entry, in bytes
        length: u64,
        // XXX Missing 'attrib's entirely
        // These ones are optional / vary by partition scheme.  These are the GPT ones:
        /// If provided by scheme (e.g., GPT): a label associated with this partition entry
        label: Option<String>,
        /// If provided by scheme (e.g., GPT, MBR): the raw value that was decoded to the `::type_`
        /// alias.  String representation varies by the specific scheme implementation.
        rawtype: Option<String>,
        /// If provided by scheme (e.g., GPT): a unique identifier (UUID, GUID) for this partition.
        /// These are generated randomly when partitions are created, and are unique unless cloned
        /// or intentionally duplicated.
        rawuuid: Option<String>,
        /// If provided by scheme (e.g., GPT, MBR): The EFI path to this partition.  E.g.,
        /// `HD(1,GPT,12345678-9abc-...,0x80,0xc8)` (GPT) or `HD(2,MBR,0x12345678,0x100,0x100)`
        /// (MBR).
        efimedia: Option<String>,
    },
    /// `EdgeMetadata::LABEL` is metadata associated with the `Edge` between a `GeomClass::LABEL`
    /// and some lower `Geom` in the tree.
    ///
    /// It is mostly a vestigial implementation detail of FreeBSD's LABEL GEOM class.
    LABEL {
        /// Always zero
        index: u64,
        /// Always zero
        offset: u64,
        /// The `length` of the volume represented by this label, in bytes
        length: u64,
        /// `length` divided by 512
        seclength: u64,
        /// Always zero
        secoffset: u64,
    },
}

impl EdgeMetadata {
    fn disk_from_raw(p: &raw::Provider) -> Result<Box<EdgeMetadata>, Error> {
        let raw = &p.config;
        Ok(Box::new(Self::DISK {
            fwheads: raw.fwheads.ok_or(Error::GraphError)?,
            fwsectors: raw.fwsectors.ok_or(Error::GraphError)?,
            rotationrate: raw.rotationrate.ok_or(Error::GraphError)?,
            ident: raw.ident.as_ref().ok_or(Error::GraphError)?.to_owned(),
            lunid: raw.lunid.as_ref().ok_or(Error::GraphError)?.to_owned(),
            descr: raw.descr.as_ref().ok_or(Error::GraphError)?.to_owned(),
        }))
    }

    fn part_from_raw(p: &raw::Provider) -> Result<Box<EdgeMetadata>, Error> {
        let raw = &p.config;
        Ok(Box::new(Self::PART {
            start: raw.start.ok_or(Error::GraphError)?,
            end: raw.end.ok_or(Error::GraphError)?,
            index: raw.index.ok_or(Error::GraphError)?,
            type_: raw.type_.as_ref().ok_or(Error::GraphError)?.to_owned(),
            offset: raw.offset.ok_or(Error::GraphError)?,
            length: raw.length.ok_or(Error::GraphError)?,

            label:       raw.label.as_ref().map(|v| v.to_owned()),
            rawtype:   raw.rawtype.as_ref().map(|v| v.to_owned()),
            rawuuid:   raw.rawuuid.as_ref().map(|v| v.to_owned()),
            efimedia: raw.efimedia.as_ref().map(|v| v.to_owned()),
        }))
    }

    fn label_from_raw(p: &raw::Provider) -> Result<Box<EdgeMetadata>, Error> {
        let raw = &p.config;
        Ok(Box::new(Self::LABEL {
            index: raw.index.ok_or(Error::GraphError)?,
            offset: raw.offset.ok_or(Error::GraphError)?,
            length: raw.length.ok_or(Error::GraphError)?,
            seclength: raw.seclength.ok_or(Error::GraphError)?,
            secoffset: raw.secoffset.ok_or(Error::GraphError)?,
        }))
    }
}

/// An `Edge` connects two `Geom`s in a tree.
///
/// In GEOM terminology, it represents a Consumer-Provider pair.
#[derive(Debug)]
pub struct Edge {
    /// The name of the `Edge`, established by the "provider" (associated with the parent `Geom`).
    ///
    /// Not identical to the parent `Geom`'s name; for example, `GeomClass::PART` geoms will have
    /// names the represent the entire partition table, but individual `Edge`s from them will have
    /// names specific to a single partition entry.
    pub name: String,
    /// GEOM internal access reference counts
    pub mode: Mode,
    /// The size of the logical volume represented, in bytes
    pub mediasize: u64,
    /// The native sector size of the underlying volume, in bytes
    pub sectorsize: u64,
    /// The "stripe size" of the underlying media, in bytes (if any; may be zero)
    pub stripesize: u64,
    pub stripeoffset: u64,
    /// Metadata for `Edge`s descending from `DISK`, `PART`, or `LABEL` `Geom`s.
    pub metadata: Option<Box<EdgeMetadata>>,

    /// Child, or consumer `Geom`.
    pub consumer_geom: NodeId,
    /// Parent, or provider `Geom`.
    pub provider_geom: NodeId,
}

/// A unique identifier for a `Geom` in a `Graph`.
pub type NodeId = u64;
/// A unique identifier for an `Edge` in a `Graph`.
pub type EdgeId = (u64, u64);

/// A `geom::Graph` represents a snapshot of the GEOM state of a FreeBSD instance.
///
/// (Math jargon: It is actually a "forest" of disconnected components, rather than a "graph," and
/// those components form "trees.")
#[derive(Debug)]
pub struct Graph {
    /// Contains all of the `Geom`s in the forest
    pub nodes: BTreeMap<NodeId, Geom>,
    /// Contains all of the `Edge`s in the forest
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Represents the out-edges of each `Geom`, by id
    pub outedges: BTreeMap<NodeId, Vec<EdgeId>>,
    /// Represents the in-edges of each `Geom`, by id
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

    /// Returns an `Iterator` which yields each `(&NodeId, &Geom)` for roots (i.e., `rank` 1).
    pub fn roots_iter(&self) -> RootsIter {
        RootsIter { iter: self.nodes.iter() }
    }

    /// Given the `NodeId` of a `Geom`, returns an `Iterator` which yields each `EdgeId` descending
    /// from the node.
    pub fn child_edgeids_iter(&self, id: &NodeId) -> ChildEdgeIdsIter {
        let v = self.inedges.get(&id);
        ChildEdgeIdsIter {
            iter: match v {
                Some(edges) => Some(edges.iter()),
                None => None,
            }
        }
    }

    /// Given the `NodeId` of a `Geom`, returns an `Iterator` which yields each `(&EdgeId, &Edge)`
    /// descending from the node.
    pub fn child_edges_iter(&self, id: &NodeId) -> ChildEdgesIter {
        ChildEdgesIter {
            edges: &self.edges,
            iter: self.child_edgeids_iter(id),
        }
    }

    /// Given the `NodeId` of a Geom`, returns an `Iterator` which yields each `(&EdgeId, &Edge,
    /// &Geom)` descending from the node.
    pub fn child_geoms_iter(&self, id: &NodeId) -> ChildGeomsIter {
        ChildGeomsIter {
            nodes: &self.nodes,
            iter: self.child_edges_iter(id),
        }
    }
}

#[derive(Debug)]
pub struct RootsIter<'a> {
    iter: std::collections::btree_map::Iter<'a, NodeId, Geom>,
}

impl<'a> Iterator for RootsIter<'a> {
    type Item = (&'a NodeId, &'a Geom);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.iter.next();
            if next.is_none() {
                return next;
            }
            if let Some(kv) = next {
                if kv.1.rank == 1 {
                    return next;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ChildEdgeIdsIter<'a> {
    iter: Option<std::slice::Iter<'a, EdgeId>>,
}

impl<'a> Iterator for ChildEdgeIdsIter<'a> {
    type Item = &'a EdgeId;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iter {
            None => None,
            Some(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
pub struct ChildEdgesIter<'a> {
    edges: &'a BTreeMap<EdgeId, Edge>,
    iter: ChildEdgeIdsIter<'a>,
}

impl<'a> Iterator for ChildEdgesIter<'a> {
    type Item = (&'a EdgeId, &'a Edge);

    fn next(&mut self) -> Option<Self::Item> {
        match &self.iter.next() {
            None => None,
            Some(edgeid) => Some((edgeid, self.edges.get(edgeid).unwrap())),
        }
    }
}

#[derive(Debug)]
pub struct ChildGeomsIter<'a> {
    nodes: &'a BTreeMap<NodeId, Geom>,
    iter: ChildEdgesIter<'a>,
}

impl<'a> Iterator for ChildGeomsIter<'a> {
    type Item = (&'a EdgeId, &'a Edge, &'a Geom);

    fn next(&mut self) -> Option<Self::Item> {
        match &self.iter.next() {
            None => None,
            Some((edgeid, edge)) =>
                Some((edgeid, edge, self.nodes.get(&edge.consumer_geom).unwrap())),
        }
    }
}

fn scan_ptr(s: &str) -> Result<u64, Error> {
    let p = scan_fmt!(s, "{x}", [hex u64])?;
    return Ok(p)
}

// XXX double check that all providers are attached to a consumer, but I think they are via DEV.
/// Converts a logical GEOM forest from the unprocessed, `geom::raw::Mesh` format to the more
/// convenient and strongly-typed `geom::Graph` format.
pub fn decode_graph(mesh: &raw::Mesh) -> Result<Graph, Error> {
    let mut result = Graph::new();

    // First pass: create nodes; temp: collect consumers, producers, and their pairs.
    let mut cons: BTreeMap<u64, &raw::Consumer> = BTreeMap::new();
    let mut provs: BTreeMap<u64, &raw::Provider> = BTreeMap::new();
    let mut conprods: BTreeSet<EdgeId> = BTreeSet::new();

    for class in &mesh.classes {
        let classkind = GeomClass::from_str(&class.name)?;

        for geom in &class.geoms {
            let geom_id = scan_ptr(&geom.id)?;
            let mut config = None;
            if classkind == GeomClass::PART {
                let rawconfig = &geom.config.as_ref()
                    .ok_or(Error::GraphError)?;
                let partscheme = PartScheme::from_str(&rawconfig.scheme
                    .as_ref()
                    .ok_or(Error::GraphError)?)?;
                let partstate = PartState::from_str(&rawconfig.state
                    .as_ref()
                    .ok_or(Error::GraphError)?)?;

                config = Some(Box::new(PartMetadata {
                    scheme: partscheme,
                    state: partstate,
                    entries:
                        rawconfig.entries.ok_or(Error::GraphError)?,
                    first:
                        rawconfig.first.ok_or(Error::GraphError)?,
                    last:
                        rawconfig.last.ok_or(Error::GraphError)?,
                    fwsectors:
                        rawconfig.fwsectors.ok_or(Error::GraphError)?,
                    fwheads:
                        rawconfig.fwheads.ok_or(Error::GraphError)?,
                    modified:
                        rawconfig.modified.ok_or(Error::GraphError)?,
                }));
            }
            result.nodes.insert(geom_id, Geom {
                class: classkind,
                name: geom.name.to_owned(),
                rank: geom.rank,
                metadata: config,
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
        let rawcons = cons.get(&cid).ok_or(Error::GraphError)?;
        let rawprov = provs.get(&pid).ok_or(Error::GraphError)?;
        // DEV geoms consume providers with access r0w0e0; allow it.
        // I guess it is technically possible for non-zero consumers to share providers, in which
        // case their access would sum to the provider's?  Maybe we should just track both values.
        if &rawcons.mode != &rawprov.mode && &rawcons.mode != "r0w0e0" {
            return Err(Error::GraphError);
        }

        // Geom associated with the provider in this pair.
        let provgeom_id = scan_ptr(&rawprov.geom_ref.ref_)?;
        let provgeom = result.nodes.get(&provgeom_id).ok_or(Error::GraphError)?;
        // And consumer.
        let consgeom_id = scan_ptr(&rawcons.geom_ref.ref_)?;

        let edge = Edge {
            name: rawprov.name.to_owned(),
            mode: Mode::from_str(&rawprov.mode)?,
            mediasize: rawprov.mediasize,
            sectorsize: rawprov.sectorsize,
            stripesize: rawprov.stripesize,
            stripeoffset: rawprov.stripeoffset,
            metadata: match provgeom.class {
                GeomClass::DISK => Some(EdgeMetadata::disk_from_raw(rawprov)?),
                GeomClass::PART => Some(EdgeMetadata::part_from_raw(rawprov)?),
                GeomClass::LABEL => Some(EdgeMetadata::label_from_raw(rawprov)?),
                _ => None,
            },
            consumer_geom: consgeom_id,
            provider_geom: provgeom_id,
        };

        let edge_id = (*cid, *pid);
        result.edges.insert(edge_id, edge);

        let invec = result.inedges.entry(provgeom_id).or_insert(Vec::new());
        (*invec).push(edge_id);

        let outvec = result.outedges.entry(consgeom_id).or_insert(Vec::new());
        (*outvec).push(edge_id);
    }

    return Ok(result);
}

#[cfg(test)]
mod tests {
    use crate::{raw, graph};
    const SAMPLE_XML: &str = include_str!("test/fullsample.xml");

    #[test]
    fn large_sample_decode() {
        let rawmesh = raw::parse_xml(&SAMPLE_XML).unwrap();
        graph::decode_graph(&rawmesh).unwrap();
    }

    #[test]
    fn roots_iterator() {
        let rawmesh = raw::parse_xml(&SAMPLE_XML).unwrap();
        let g = graph::decode_graph(&rawmesh).unwrap();

        for (_, root) in g.roots_iter() {
            assert_eq!(root.class, graph::GeomClass::DISK);
        }
    }
}
