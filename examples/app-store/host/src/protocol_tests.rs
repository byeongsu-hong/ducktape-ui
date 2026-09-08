//! Actual SDK packages cross the same admission path at install and hot reload.
use super::*;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let path =
            std::env::temp_dir().join(format!("ice-protocol-{:032x}", u128::from_le_bytes(random)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(native: bool) -> CatalogEntry {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/native-reload-fixture"
    } else {
        "../target/reload-v1-fixture"
    });
    crate::catalog::scan_dir(&path)
        .into_iter()
        .next()
        .expect("build current reload-v1 fixture")
}

fn input(node: &wire::Node) -> Option<(u32, String)> {
    if let wire::Node::Input {
        key,
        on_input,
        value,
        ..
    } = node
        && key.ends_with("/draft")
    {
        return Some((*on_input, value.clone()));
    }
    node.children().iter().find_map(input)
}

fn draft(guest: &mut Guest, text: Option<&str>) -> String {
    let events = text
        .map(|text| {
            vec![wire::Event::Input {
                handler: input(guest.frame.root.as_ref().unwrap()).unwrap().0,
                text: text.into(),
            }]
        })
        .unwrap_or_default();
    let mut frame = guest.tick_inner(&wire::encode(&events)).unwrap();
    merge(&mut guest.frame.root, &mut frame).unwrap();
    guest.frame = frame;
    input(guest.frame.root.as_ref().unwrap()).unwrap().1
}

fn different_epoch() -> u32 {
    wire::WIRE_EPOCH
        .checked_add(1)
        .filter(|next| next.ilog10() == wire::WIRE_EPOCH.ilog10())
        .unwrap_or(wire::WIRE_EPOCH - 1)
}

fn incompatible(original: &CatalogEntry, scratch: &Scratch, native: bool) -> CatalogEntry {
    let manifest = |text: &[u8]| {
        let text = std::str::from_utf8(text).unwrap();
        let (prefix, _) = text.rsplit_once('\n').unwrap();
        format!("{prefix}\n{}", different_epoch()).into_bytes()
    };
    if native {
        let (old, executable) =
            crate::catalog::native_package(std::path::Path::new(&original.path)).unwrap();
        let path = scratch.0.join("candidate.native");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("manifest"), manifest(&old)).unwrap();
        std::fs::write(
            path.join(if cfg!(windows) { "app.exe" } else { "app" }),
            executable,
        )
        .unwrap();
    } else {
        let mut bytes = std::fs::read(&original.path).unwrap();
        let current = wire::manifest::read_manifest(&bytes).unwrap();
        let epoch = current.wire_epoch.to_string();
        let (start, old) = wasmparser::Parser::new(0)
            .parse_all(&bytes)
            .find_map(|payload| match payload.unwrap() {
                wasmparser::Payload::CustomSection(section)
                    if section.name() == wire::manifest::MANIFEST_SECTION =>
                {
                    Some((section.data_offset(), section.data()))
                }
                _ => None,
            })
            .unwrap();
        let end = start + old.len();
        let begin = end - epoch.len();
        assert_eq!(&bytes[begin..end], epoch.as_bytes());
        let replacement = different_epoch().to_string();
        assert_eq!(replacement.len(), epoch.len());
        bytes[begin..end].copy_from_slice(replacement.as_bytes());
        assert_eq!(
            wire::manifest::read_manifest(&bytes).unwrap().wire_epoch,
            different_epoch()
        );
        std::fs::write(scratch.0.join("candidate.wasm"), bytes).unwrap();
    }
    let mut entry = crate::catalog::scan_dir(&scratch.0).remove(0);
    entry.id = original.id.clone();
    entry
}

fn check(native: bool, reload: bool) {
    let original = fixture(native);
    let scratch = Scratch::new();
    let candidate = incompatible(&original, &scratch, native);
    let expected = format!(
        "wire epoch guest {}, host {}",
        different_epoch(),
        wire::WIRE_EPOCH
    );
    if !reload {
        assert_eq!(
            Guest::load(&candidate).expect_err("incompatible guest initialized"),
            expected
        );
        let mut control = Guest::load(&original).unwrap();
        assert_eq!(draft(&mut control, None), "");
        return;
    }

    let mut guest = Guest::load(&original).unwrap();
    assert_eq!(draft(&mut guest, None), "");
    assert_eq!(
        draft(&mut guest, Some("retain my draft")),
        "retain my draft"
    );
    let before = guest.backend.snapshot().unwrap();
    let alive = guest.alive.clone();
    let running = Running {
        id: original.id.clone(),
        name: original.name.clone(),
        window: iced::window::Id::unique(),
        surface: Surface(Arc::new(Mutex::new(guest))),
    };
    let reload =
        iced::futures::executor::block_on(prepare_reload(candidate, vec![running.clone()], 1));
    let error = reload::finish_reload(std::slice::from_ref(&running), 1, reload).unwrap_err();
    assert_eq!(error.message, expected);
    let mut guest = running.surface.0.lock().unwrap();
    assert!(
        Arc::ptr_eq(&guest.alive, &alive),
        "rejected candidate displaced the live instance"
    );
    assert_eq!(guest.entry.hash, original.hash);
    assert_eq!(guest.backend.snapshot().unwrap(), before);
    assert_eq!(draft(&mut guest, Some("still editable")), "still editable");
}

#[test]
#[ignore = "requires current bundled reload-v1 fixture"]
fn wire_epoch_wasm_reload_rejects_before_execution() {
    check(false, true);
}

#[test]
#[ignore = "requires current native reload-v1 fixture"]
fn wire_epoch_native_reload_rejects_before_execution() {
    check(true, true);
}

#[test]
#[ignore = "requires current bundled reload-v1 fixture"]
fn wire_epoch_wasm_install_rejects_before_execution() {
    check(false, false);
}

#[test]
#[ignore = "requires current native reload-v1 fixture"]
fn wire_epoch_native_install_rejects_before_execution() {
    check(true, false);
}

#[test]
fn wire_epoch_wasm_rejects_before_instantiation() {
    let scratch = Scratch::new();
    let path = scratch.0.join("start-trap.wasm");
    let component = wat::parse_str(
        r#"(component
            (type $bytes (list u8))
            (type $snapshot (result $bytes (error string)))
            (type $restored (result (error string)))
            (core module $m
                (memory (export "memory") 1)
                (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
                (func (export "init") (param i32) unreachable)
                (func (export "tick") (param i32 i32) (result i32) unreachable)
                (func (export "snapshot") (result i32) unreachable)
                (func (export "restore") (param i32 i32 i32) (result i32) unreachable)
                (func $start unreachable) (start $start))
            (core instance $i (instantiate $m))
            (func (export "init") (param "macos" bool)
                (canon lift (core func $i "init")))
            (func (export "tick") (param "events" $bytes) (result $bytes)
                (canon lift (core func $i "tick") (memory $i "memory") (realloc (func $i "realloc"))))
            (func (export "snapshot") (result $snapshot)
                (canon lift (core func $i "snapshot") (memory $i "memory") (realloc (func $i "realloc"))))
            (func (export "restore") (param "state" $bytes) (param "macos" bool) (result $restored)
                (canon lift (core func $i "restore") (memory $i "memory") (realloc (func $i "realloc")))))"#
    ).unwrap();
    let write = |epoch| {
        let mut bytes = component.clone();
        let section_name = wire::manifest::MANIFEST_SECTION;
        let manifest = format!("ice.manifest.v2\nProbe\n\n\nnone\n{epoch}");
        let section_len = 1 + section_name.len() + manifest.len();
        assert!(section_len < 128);
        bytes.extend_from_slice(&[0, section_len as u8, section_name.len() as u8]);
        bytes.extend_from_slice(section_name.as_bytes());
        bytes.extend_from_slice(manifest.as_bytes());
        std::fs::write(&path, bytes).unwrap();
        crate::catalog::scan_dir(&scratch.0).remove(0)
    };
    let entry = write(different_epoch());
    assert_eq!(
        Instance::new(&entry)
            .err()
            .expect("incompatible component ran"),
        format!(
            "wire epoch guest {}, host {}",
            different_epoch(),
            wire::WIRE_EPOCH
        )
    );
    let compatible = write(wire::WIRE_EPOCH);
    // Use the admitted component directly to establish that its core start is
    // valid Wasm and executes the sentinel before any guest export is called.
    let (component, _) = super::component(&compatible).unwrap();
    let mut store = new_wasm_store();
    let linker = Linker::new(engine());
    ViewPre::new(linker.instantiate_pre(&component).unwrap())
        .expect("sentinel exports must satisfy the production WIT");
    arm(&mut store);
    let error = linker
        .instantiate(&mut store, &component)
        .expect_err("start sentinel did not trap");
    assert!(format!("{error:#}").contains("unreachable"), "{error:#}");
}
