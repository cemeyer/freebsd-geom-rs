# freebsd-geom-rs
A Rust library for inspecting the [GEOM(4)](https://www.freebsd.org/cgi/man.cgi?query=geom&sektion=4) graph

## Example

```rust
use freebsd_geom as geom;

// Pull the current GEOM graph out of the kernel and parse it into a graph structure.
let mygraph = geom::get_graph()?;

// Print the name of the root of each independent object tree (i.e., DISK devices).
for (_, root) in mygraph.roots_iter() {
    println!("Root: {} (type {:?})", root.name, root.class);
}
```

That might print something like:

```
Root: ada0 (type DISK)
Root: nvd0 (type DISK)
```

We can recursively iterate all descendents of a node:

```rust
for (rootid, root) in mygraph.roots_iter() {
    println!("Root {} descendents:", root.name);
    for (_, _, desc) in mygraph.descendents_iter(rootid) {
        println!("{}Name: {} Class: {:?}", " ".repeat(desc.rank - 1), desc.name, desc.class);
    }
}
```

That might print something like:

```
Root ada0 descendents:
 Name: ada0 Class: DEV
 Name: ada0 Class: PART
  Name: ada0p1 Class: DEV
  Name: ada0p1 Class: LABEL  
   Name: gpt/foobar Class: DEV
   Name: ffs.gpt/foobar Class: VFS
```

We can look for all partition tables (type PART) and print out their partitions:

```rust
for (rootid, _) in mygraph.roots_iter() {
    for (_, parent_edge, desc) in mygraph.descendents_iter(rootid) {
        if desc.class == geom::GeomClass::PART {
            println!("Partitions of {} (scheme: {:?}):", desc.name, desc.metadata.as_ref().unwrap().scheme);
            
            for (_, edge) in mygraph.child_edges_iter(&parent_edge.consumer_geom) {
                // All PART child edges will have EdgeMetadata::PART, but Rust doesn't know that.
                if let geom::EdgeMetadata::PART { type_: ptype, label: plabel, rawuuid: puuid, .. } =
                    edge.metadata.as_ref().unwrap().as_ref() {
                    println!("  {} (type: {} label: '{}' uuid: {}",
                                edge.name, ptype, plabel.as_ref().unwrap_or(&"<None>".to_owned()),
                                puuid.as_ref().unwrap_or(&"<None>".to_owned()))
                }
            }
        }
    }
}
```

This might print something like:

```
Partitions of nvd1 (scheme: GPT):
  nvd1p2 (type: freebsd-ufs label: 'my-fs-label' uuid: abcdef01-79e7-11e9-b158-7085c25400ea
  nvd1p1 (type: freebsd-swap label: 'my-swap-label' uuid: 23456789-79e7-11e9-b158-7085c25400ea
...
```
