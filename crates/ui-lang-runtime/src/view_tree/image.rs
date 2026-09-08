//! Raster pictures are decoded once, within the guest's picture allowance.
use super::*;
use ::image::{ImageDecoder, ImageReader};
use std::io::Cursor;

const MAX_IMAGE_PIXELS: u64 = 4 * 1024 * 1024;
pub(super) const MAX_FRAME_PIXELS: u64 = 8 * 1024 * 1024;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
pub(super) const MAX_ENTRIES: usize = 8192;

impl Pictures {
    pub(super) fn keep_image(&mut self, hash: u64, data: &wire::ImageData, frame: &mut u64) {
        if self.images.contains_key(&hash)
            || self.images.len() + self.handles.len() >= MAX_ENTRIES
            || data.byte_len() > MAX_PICTURE_BYTES - self.bytes
        {
            return;
        }
        self.bytes += data.byte_len();
        // Remember failures as well: repeating a malformed payload does not
        // cause repeated decoding. Entries and bytes both have lifetime caps.
        let initial = MAX_IMAGE_PIXELS.min(*frame).min(MAX_PIXELS - self.pixels);
        let mut allowance = initial;
        let decoded = decode(data, &mut allowance);
        let spent = initial - allowance;
        *frame -= spent;
        self.pixels += spent;
        let handle = decoded.map(|(handle, _)| handle);
        self.images.insert(hash, handle);
    }
}

fn decode(data: &wire::ImageData, allowance: &mut u64) -> Option<(widget::image::Handle, u64)> {
    let mut admit = |width: u32, height: u32| {
        let pixels = u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || pixels > *allowance {
            return None;
        }
        *allowance -= pixels;
        Some(pixels)
    };
    match data {
        wire::ImageData::Rgba {
            width,
            height,
            pixels,
        } => {
            let count = admit(*width, *height)?;
            if count.checked_mul(4)? != pixels.len() as u64 {
                return None;
            }
            Some((
                widget::image::Handle::from_rgba(*width, *height, pixels.clone()),
                count,
            ))
        }
        wire::ImageData::Encoded(bytes) => {
            let mut reader = ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .ok()?;
            let mut limits = ::image::Limits::default();
            limits.max_image_width = Some(MAX_IMAGE_PIXELS as u32);
            limits.max_image_height = Some(MAX_IMAGE_PIXELS as u32);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let mut decoder = reader.into_decoder().ok()?;
            let (width, height) = decoder.dimensions();
            let count = admit(width, height)?;
            let orientation = decoder.orientation().ok()?;
            let mut pixels = ::image::DynamicImage::from_decoder(decoder).ok()?;
            pixels.apply_orientation(orientation);
            let pixels = pixels.into_rgba8();
            Some((
                widget::image::Handle::from_rgba(
                    pixels.width(),
                    pixels.height(),
                    pixels.into_raw(),
                ),
                count,
            ))
        }
    }
}

pub(super) fn render(node: &wire::Node, pictures: &Pictures) -> IceElement<'static, Output> {
    let wire::Node::Image {
        key,
        hash,
        label,
        fit,
        rotation,
        opacity,
        filter,
        width,
        height,
        ..
    } = node
    else {
        unreachable!()
    };
    let picture: IceElement<'static, Output> =
        match pictures.images.get(hash).and_then(Option::as_ref) {
            Some(handle) => {
                let mut image = widget::image(handle.clone());
                if let Some(width) = width {
                    image = image.width(length(*width));
                }
                if let Some(height) = height {
                    image = image.height(length(*height));
                }
                if let Some(fit) = fit {
                    image = image.content_fit(match fit {
                        wire::ContentFit::Contain => iced::ContentFit::Contain,
                        wire::ContentFit::Cover => iced::ContentFit::Cover,
                        wire::ContentFit::Fill => iced::ContentFit::Fill,
                        wire::ContentFit::None => iced::ContentFit::None,
                        wire::ContentFit::ScaleDown => iced::ContentFit::ScaleDown,
                    });
                }
                if let Some(rotation) = rotation {
                    image = image.rotation(match *rotation {
                        wire::Rotation::Floating(radians) => {
                            iced::Rotation::Floating(iced::Radians(radians))
                        }
                        wire::Rotation::Solid(radians) => {
                            iced::Rotation::Solid(iced::Radians(radians))
                        }
                    });
                }
                if let Some(opacity) = opacity {
                    image = image.opacity(*opacity);
                }
                image
                    .filter_method(match filter {
                        wire::ImageFilter::Linear => widget::image::FilterMethod::Linear,
                        wire::ImageFilter::Nearest => widget::image::FilterMethod::Nearest,
                    })
                    .into()
            }
            None => {
                let mut space = widget::Space::new();
                if let Some(width) = width {
                    space = space.width(length(*width));
                }
                if let Some(height) = height {
                    space = space.height(length(*height));
                }
                space.into()
            }
        };
    accessible(picture, StableId::new(key), Role::Image)
        .logical_id_maybe(cfg!(test).then_some(key.as_str()))
        .label(label.clone().unwrap_or_default())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoded_pixels_obey_the_same_admission_as_rgba() {
        use ::image::ImageEncoder;
        let pixels = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let mut bytes = Vec::new();
        ::image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixels, 2, 1, ::image::ExtendedColorType::Rgba8)
            .unwrap();
        let data = wire::ImageData::Encoded(bytes);
        assert!(
            decode(&data, &mut 1).is_none(),
            "encoded dimensions must be checked before pixel decode"
        );
        let (handle, count) = decode(&data, &mut 2).unwrap();
        assert_eq!(count, 2);
        let widget::image::Handle::Rgba {
            width,
            height,
            pixels: actual,
            ..
        } = handle
        else {
            panic!("host must retain decoded RGBA, not defer another decode to the renderer")
        };
        assert_eq!((width, height), (2, 1));
        assert_eq!(actual.as_ref(), pixels);
    }

    #[test]
    fn jpeg_orientation_matches_the_native_image_loader() {
        let mut jpeg = Vec::new();
        ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
            .encode(
                &[255, 0, 0, 0, 0, 255],
                2,
                1,
                ::image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        for orientation in 1u8..=8 {
            // One little-endian TIFF IFD: Orientation, SHORT, count 1.
            let exif = [
                b'E',
                b'x',
                b'i',
                b'f',
                0,
                0,
                b'I',
                b'I',
                42,
                0,
                8,
                0,
                0,
                0,
                1,
                0,
                0x12,
                1,
                3,
                0,
                1,
                0,
                0,
                0,
                orientation,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
            let mut bytes = jpeg[..2].to_vec();
            bytes.extend_from_slice(&[0xff, 0xe1, 0, 34]);
            bytes.extend_from_slice(&exif);
            bytes.extend_from_slice(&jpeg[2..]);
            let expected = iced::advanced::graphics::image::load(
                &widget::image::Handle::from_bytes(bytes.clone()),
            )
            .unwrap();
            let (actual, _) = decode(&wire::ImageData::Encoded(bytes), &mut 2).unwrap();
            let widget::image::Handle::Rgba {
                width,
                height,
                pixels,
                ..
            } = actual
            else {
                panic!()
            };
            assert_eq!(
                (width, height),
                expected.dimensions(),
                "EXIF orientation {orientation} dimensions"
            );
            assert_eq!(
                pixels.as_ref(),
                expected.as_raw().as_ref(),
                "EXIF orientation {orientation} pixels"
            );
        }
    }

    #[test]
    fn corrupt_pixels_spend_admitted_decode_work() {
        use ::image::ImageEncoder;
        let mut bytes = Vec::new();
        ::image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&[255; 8], 2, 1, ::image::ExtendedColorType::Rgba8)
            .unwrap();
        let at = bytes.windows(4).position(|chunk| chunk == b"IDAT").unwrap() + 6;
        bytes[at] ^= 0xff;
        let mut pictures = Pictures::default();
        let mut frame = 3;
        pictures.keep_image(1, &wire::ImageData::Encoded(bytes), &mut frame);
        assert!(
            pictures.images[&1].is_none(),
            "the corrupted PNG must fail decoding"
        );
        assert_eq!(
            frame, 1,
            "failed pixel decoding still spends the frame work allowance"
        );
        assert_eq!(pictures.pixels, 2);
    }

    #[test]
    fn malformed_and_over_budget_images_are_not_admitted() {
        let rgba = wire::ImageData::Rgba {
            width: 2,
            height: 1,
            pixels: vec![255; 8],
        };
        assert!(decode(&rgba, &mut 1).is_none());
        assert!(decode(&rgba, &mut 2).is_some());
        let mut ample = MAX_IMAGE_PIXELS;
        assert!(decode(&wire::ImageData::Encoded(vec![0; 32]), &mut ample).is_none());
        let mut largest = u64::MAX;
        assert!(
            decode(
                &wire::ImageData::Rgba {
                    width: u32::MAX,
                    height: u32::MAX,
                    pixels: vec![]
                },
                &mut largest
            )
            .is_none()
        );
    }
    #[test]
    fn raster_admission_shares_svg_byte_and_entry_caps() {
        let rgba = wire::ImageData::Rgba {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        let mut full_bytes = Pictures {
            bytes: MAX_PICTURE_BYTES - 3,
            ..Pictures::default()
        };
        let mut frame = MAX_FRAME_PIXELS;
        full_bytes.keep_image(1, &rgba, &mut frame);
        assert!(
            full_bytes.images.is_empty(),
            "raster bytes share the SVG lifetime allowance"
        );
        let mut full_entries = Pictures::default();
        for hash in 0..MAX_ENTRIES as u64 {
            full_entries.keep(hash, b"");
        }
        assert_eq!(full_entries.handles.len(), MAX_ENTRIES);
        full_entries.keep_image(1, &rgba, &mut frame);
        assert!(
            full_entries.images.is_empty(),
            "zero-byte SVGs cannot bypass the shared entry cap"
        );
    }

    #[test]
    fn picture_admission_is_shared_and_failures_are_retained() {
        let rgba = wire::ImageData::Rgba {
            width: 2,
            height: 1,
            pixels: vec![255; 8],
        };
        let mut pictures = Pictures::default();
        let mut frame = 3;
        pictures.keep_image(1, &rgba, &mut frame);
        pictures.keep_image(2, &rgba, &mut frame);
        assert!(pictures.images[&1].is_some());
        assert!(pictures.images[&2].is_none());
        assert_eq!(pictures.pixels, 2);
        assert_eq!(frame, 1);
        let mut next_frame = MAX_FRAME_PIXELS;
        pictures.keep_image(2, &rgba, &mut next_frame);
        assert!(pictures.images[&2].is_none());
        assert_eq!(pictures.bytes, 16);
    }
}
