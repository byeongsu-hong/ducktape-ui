//! Native zoom and pan over an image admitted by the shared picture cache.
use super::*;

pub(super) fn render(node: &wire::Node, pictures: &Pictures) -> IceElement<'static, Output> {
    let wire::Node::ImageViewer {
        key,
        hash,
        label,
        fit,
        filter,
        width,
        height,
        options,
        ..
    } = node
    else {
        unreachable!()
    };
    let picture: IceElement<'static, Output> =
        match pictures.images.get(hash).and_then(Option::as_ref) {
            Some(handle) => {
                let mut viewer = widget::image::viewer(handle.clone());
                if let Some(width) = width {
                    viewer = viewer.width(length(*width));
                }
                if let Some(height) = height {
                    viewer = viewer.height(length(*height));
                }
                if let Some(fit) = fit {
                    viewer = viewer.content_fit(match fit {
                        wire::ContentFit::Contain => iced::ContentFit::Contain,
                        wire::ContentFit::Cover => iced::ContentFit::Cover,
                        wire::ContentFit::Fill => iced::ContentFit::Fill,
                        wire::ContentFit::None => iced::ContentFit::None,
                        wire::ContentFit::ScaleDown => iced::ContentFit::ScaleDown,
                    });
                }
                if let Some(padding) = options.padding {
                    viewer = viewer.padding(padding);
                }
                if let Some((min, max)) = options.scale_bounds {
                    viewer = viewer.min_scale(min).max_scale(max);
                }
                if let Some(step) = options.scale_step {
                    viewer = viewer.scale_step(step);
                }
                viewer
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
    fn viewer_uses_the_existing_raster_cache_and_admission_budget() {
        let data = wire::ImageData::Rgba {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        let mut pictures = Pictures::default();
        let mut frame = super::super::image::MAX_FRAME_PIXELS;
        pictures.keep_image(7, &data, &mut frame);
        let mut node = wire::Node::ImageViewer {
            key: "photo".into(),
            hash: 7,
            data: None,
            label: None,
            fit: None,
            filter: Default::default(),
            width: None,
            height: None,
            options: Default::default(),
        };
        pictures.adopt(&node);
        assert!(
            pictures.images.get(&7).is_some_and(Option::is_some),
            "a viewer must reuse the already decoded raster handle"
        );
        assert_eq!(pictures.bytes, 4);
        if let wire::Node::ImageViewer {
            hash,
            data: payload,
            ..
        } = &mut node
        {
            *hash = 8;
            *payload = Some(data.clone());
        }
        pictures.adopt(&node);
        assert!(
            pictures.images.get(&8).is_some_and(Option::is_some),
            "new viewer data must enter the shared raster cache"
        );
        assert_eq!(pictures.bytes, 8);
        if let wire::Node::ImageViewer {
            hash,
            data: payload,
            ..
        } = &mut node
        {
            *hash = 10;
            *payload = Some(wire::ImageData::Encoded(vec![0; 4]));
        }
        pictures.adopt(&node);
        assert!(
            pictures.images.get(&10).is_some_and(Option::is_none),
            "invalid viewer bytes must be remembered as a rejected image"
        );
        let rejected_bytes = pictures.bytes;
        pictures.adopt(&node);
        assert_eq!(
            pictures.bytes, rejected_bytes,
            "a rejected viewer hash must not be decoded again"
        );
        pictures.bytes = MAX_PICTURE_BYTES;
        if let wire::Node::ImageViewer { hash, .. } = &mut node {
            *hash = 9;
        }
        pictures.adopt(&node);
        assert!(
            !pictures.images.contains_key(&9),
            "viewer payloads must obey the same lifetime byte budget"
        );
    }
}
