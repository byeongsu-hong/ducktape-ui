//! One SVG rendered into every raster shape the three platforms ask for.
//!
//! Keeping a single vector source means the macOS `.icns`, the Windows `.ico`,
//! the freedesktop hicolor theme, and the window icon iced hands to the
//! platform cannot drift apart, and none of them is a checked-in binary.

use std::collections::BTreeMap;

/// `.icns` entry types paired with the pixel size each carries. macOS chooses
/// between the 1x and 2x members of a pair by display scale, so 256 and 512 are
/// rendered once and stored under both of their type codes.
const ICNS_ENTRIES: &[(&str, u32)] = &[
    ("ic11", 32),
    ("ic12", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic13", 256),
    ("ic09", 512),
    ("ic14", 512),
    ("ic10", 1024),
];

/// Windows reads one of these out of the `.ico` per surface: 16 for the title
/// bar, 32 for the taskbar, 48 for the desktop, 256 for the large icon view.
const ICO_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

/// The `hicolor` sizes a freedesktop desktop entry is looked up in.
pub(super) const HICOLOR_SIZES: &[u32] = &[32, 48, 64, 128, 256, 512];

pub(super) fn icns(svg: &[u8]) -> Result<Vec<u8>, String> {
    let tree = tree(svg)?;
    let mut rendered = BTreeMap::new();
    for (_, size) in ICNS_ENTRIES {
        if !rendered.contains_key(size) {
            rendered.insert(*size, png_of(&tree, *size)?);
        }
    }
    let mut body = Vec::new();
    for (kind, size) in ICNS_ENTRIES {
        let png = &rendered[size];
        let length = u32::try_from(png.len() + 8)
            .map_err(|_| format!("the {size}x{size} icon is too large for an .icns entry"))?;
        body.extend_from_slice(kind.as_bytes());
        body.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(png);
    }
    let total = u32::try_from(body.len() + 8)
        .map_err(|_| "the rendered icon is too large for an .icns file".to_owned())?;
    let mut icns = Vec::with_capacity(body.len() + 8);
    icns.extend_from_slice(b"icns");
    icns.extend_from_slice(&total.to_be_bytes());
    icns.extend_from_slice(&body);
    Ok(icns)
}

pub(super) fn ico(svg: &[u8]) -> Result<Vec<u8>, String> {
    let tree = tree(svg)?;
    let images = ICO_SIZES
        .iter()
        .map(|size| png_of(&tree, *size))
        .collect::<Result<Vec<_>, _>>()?;
    let count = u16::try_from(images.len()).map_err(|_| "too many icon sizes".to_owned())?;
    let mut offset = 6 + 16 * images.len();
    let mut ico = Vec::with_capacity(offset + images.iter().map(Vec::len).sum::<usize>());
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&count.to_le_bytes());
    for (size, png) in ICO_SIZES.iter().zip(&images) {
        // A 256-pixel entry records its size as zero; the field is one byte.
        let extent = u8::try_from(size % 256).expect("an icon size below 256 after wrapping");
        let length = u32::try_from(png.len())
            .map_err(|_| format!("the {size}x{size} icon is too large for an .ico entry"))?;
        let start =
            u32::try_from(offset).map_err(|_| "the rendered icon is too large".to_owned())?;
        ico.extend_from_slice(&[extent, extent, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&length.to_le_bytes());
        ico.extend_from_slice(&start.to_le_bytes());
        offset += png.len();
    }
    for png in images {
        ico.extend_from_slice(&png);
    }
    Ok(ico)
}

pub(super) fn png(svg: &[u8], size: u32) -> Result<Vec<u8>, String> {
    png_of(&tree(svg)?, size)
}

fn tree(svg: &[u8]) -> Result<resvg::usvg::Tree, String> {
    // The renderer is built without font support, which would drop a `<text>`
    // element instead of drawing it. Saying so beats shipping a hole.
    if svg.windows(5).any(|window| window == b"<text") {
        return Err(
            "an application icon is rendered without fonts; convert its <text> elements to paths"
                .into(),
        );
    }
    resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
        .map_err(|error| format!("cannot read the application icon: {error}"))
}

fn png_of(tree: &resvg::usvg::Tree, size: u32) -> Result<Vec<u8>, String> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| format!("cannot allocate a {size}x{size} icon"))?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| format!("cannot encode the {size}x{size} icon: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn source() -> Vec<u8> {
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg"))
            .expect("read the repository icon")
    }

    fn png_extent(png: &[u8]) -> (u32, u32) {
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
        (
            u32::from_be_bytes(png[16..20].try_into().expect("PNG width")),
            u32::from_be_bytes(png[20..24].try_into().expect("PNG height")),
        )
    }

    #[test]
    fn icns_stores_one_png_per_declared_size() {
        let icns = icns(&source()).expect("render the icon");

        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().expect("length field")) as usize,
            icns.len(),
            "the header length must cover the whole file"
        );

        let mut offset = 8;
        let mut seen = Vec::new();
        while offset < icns.len() {
            let kind = std::str::from_utf8(&icns[offset..offset + 4]).expect("entry type");
            let length = u32::from_be_bytes(
                icns[offset + 4..offset + 8]
                    .try_into()
                    .expect("entry length"),
            ) as usize;
            let (width, height) = png_extent(&icns[offset + 8..offset + length]);
            assert_eq!(width, height, "{kind} is not square");
            seen.push((kind.to_owned(), width));
            offset += length;
        }
        assert_eq!(offset, icns.len(), "entries must tile the file exactly");
        assert_eq!(
            seen,
            ICNS_ENTRIES
                .iter()
                .map(|(kind, size)| ((*kind).to_owned(), *size))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ico_directory_points_at_each_image() {
        let ico = ico(&source()).expect("render the icon");

        assert_eq!(u16::from_le_bytes(ico[0..2].try_into().unwrap()), 0);
        assert_eq!(
            u16::from_le_bytes(ico[2..4].try_into().unwrap()),
            1,
            "resource type 1 is an icon"
        );
        let count = u16::from_le_bytes(ico[4..6].try_into().unwrap()) as usize;
        assert_eq!(count, ICO_SIZES.len());

        for (index, expected) in ICO_SIZES.iter().enumerate() {
            let entry = 6 + index * 16;
            assert_eq!(
                u32::from(ico[entry]),
                expected % 256,
                "a 256-pixel entry records zero"
            );
            assert_eq!(
                u16::from_le_bytes(ico[entry + 6..entry + 8].try_into().unwrap()),
                32,
                "the images are 32-bit"
            );
            let length =
                u32::from_le_bytes(ico[entry + 8..entry + 12].try_into().unwrap()) as usize;
            let offset =
                u32::from_le_bytes(ico[entry + 12..entry + 16].try_into().unwrap()) as usize;
            assert_eq!(
                png_extent(&ico[offset..offset + length]),
                (*expected, *expected)
            );
        }
        let last = 6 + (count - 1) * 16;
        let end = u32::from_le_bytes(ico[last + 12..last + 16].try_into().unwrap()) as usize
            + u32::from_le_bytes(ico[last + 8..last + 12].try_into().unwrap()) as usize;
        assert_eq!(end, ico.len(), "the images must tile the file exactly");
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_ico_reuses_output_storage() {
        const RENDERS: usize = 16;
        const MAX_BLOCKS: u64 = 8_160;
        // Most of this is the PNG encoder, not the renderer: `encode_png`
        // clones the pixmap to demultiply it and builds a fresh deflate state
        // per image. Peak footprint stays under a megabyte; this is churn.
        const MAX_BYTES: u64 = 51_464_992;

        let source = source();
        drop(ico(&source).expect("warm the icon renderer"));

        let _profiler = dhat::Profiler::builder().testing().build();
        for _ in 0..RENDERS {
            std::hint::black_box(ico(std::hint::black_box(&source)).expect("render the icon"));
        }
        let heap = dhat::HeapStats::get();

        eprintln!(
            "{RENDERS} ICO renders: {} heap blocks / {} bytes",
            heap.total_blocks, heap.total_bytes
        );
        assert!(
            heap.total_blocks <= MAX_BLOCKS,
            "ICO rendering allocated too many blocks: {heap:?}"
        );
        assert!(
            heap.total_bytes <= MAX_BYTES,
            "ICO rendering allocated too many bytes: {heap:?}"
        );
    }

    #[test]
    fn hicolor_renders_the_size_its_directory_names() {
        for size in HICOLOR_SIZES {
            assert_eq!(
                png_extent(&png(&source(), *size).expect("render the icon")),
                (*size, *size)
            );
        }
    }

    #[test]
    fn an_icon_that_needs_a_font_is_refused() {
        let lettered = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
            <text x="2" y="12">I</text></svg>"##;
        let error = icns(lettered).expect_err("a text icon cannot render");
        assert!(error.contains("paths"), "{error}");
        let error = ico(lettered).expect_err("every format shares the refusal");
        assert!(error.contains("paths"), "{error}");
        let drawn = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
            <rect width="16" height="16" fill="#000"/></svg>"##;
        assert!(icns(drawn).is_ok(), "a drawn icon still renders");
    }
}
