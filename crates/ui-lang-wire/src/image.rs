use serde::{Deserialize, Serialize};

/// Copied raster data. Paths and native renderer allocations never cross.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageData {
    Encoded(#[serde(deserialize_with = "decode_bytes")] Vec<u8>),
    Rgba {
        width: u32,
        height: u32,
        #[serde(deserialize_with = "decode_bytes")]
        pixels: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFilter {
    #[default]
    Linear,
    Nearest,
}

impl ImageData {
    pub fn byte_len(&self) -> usize {
        match self {
            Self::Encoded(bytes) => bytes.len(),
            Self::Rgba { pixels, .. } => pixels.len(),
        }
    }

    pub fn valid_rgba(&self) -> bool {
        match self {
            Self::Encoded(_) => true,
            Self::Rgba {
                width,
                height,
                pixels,
            } => {
                *width != 0
                    && *height != 0
                    && u64::from(*width)
                        .checked_mul(u64::from(*height))
                        .and_then(|count| count.checked_mul(4))
                        == Some(pixels.len() as u64)
            }
        }
    }

    pub(crate) fn sanitize(data: &mut Option<Self>, remaining: &mut usize) {
        if let Some(value) = data {
            if !value.valid_rgba() || value.byte_len() > *remaining {
                *data = None;
            } else {
                *remaining -= value.byte_len();
            }
        }
    }
}

// A wire frame is capped at 8 MiB by hosts. Reject a collection header before
// allocating, even when callers decode ImageData directly. The smaller shared
// picture allowance is applied by frame sanitization, without truncating data.
fn decode_bytes<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    struct Bytes;
    impl<'de> serde::de::Visitor<'de> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("bounded raster bytes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            const LIMIT: usize = 8 << 20;
            if seq.size_hint().is_some_and(|size| size > LIMIT) {
                return Err(serde::de::Error::custom("raster byte limit exceeded"));
            }
            let mut bytes = Vec::new();
            while let Some(byte) = seq.next_element()? {
                if bytes.len() == LIMIT {
                    return Err(serde::de::Error::custom("raster byte limit exceeded"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_seq(Bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copied_images_roundtrip_and_refuse_malicious_collection_headers() {
        for image in [
            ImageData::Encoded(vec![0, 255]),
            ImageData::Rgba {
                width: 1,
                height: 1,
                pixels: vec![255; 4],
            },
        ] {
            assert_eq!(
                crate::decode::<ImageData>(&crate::encode(&image)).unwrap(),
                image
            );
        }
        let mut malicious = 0u32.to_le_bytes().to_vec();
        malicious.extend_from_slice(&u64::MAX.to_le_bytes());
        let error = crate::decode::<ImageData>(&malicious)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("raster byte limit"),
            "reject the header before reading elements: {error}"
        );
    }
    #[test]
    fn invalid_rgba_is_dropped_without_spending_valid_picture_budget() {
        let mut budget = 4;
        let mut invalid = Some(ImageData::Rgba {
            width: u32::MAX,
            height: u32::MAX,
            pixels: vec![0; 4],
        });
        ImageData::sanitize(&mut invalid, &mut budget);
        assert_eq!(invalid, None);
        assert_eq!(budget, 4);
        let mut valid = Some(ImageData::Rgba {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        });
        ImageData::sanitize(&mut valid, &mut budget);
        assert!(valid.is_some());
        assert_eq!(budget, 0);
        let mut excess = Some(ImageData::Encoded(vec![1]));
        ImageData::sanitize(&mut excess, &mut budget);
        assert_eq!(excess, None);
    }
}
