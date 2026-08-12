#[cfg(test)]
mod keyboard_values {
    ui_lang::include_app!("tests/cases/ui/keyboard_values.ice");

    #[test]
    fn preserves_native_keyboard_values() {
        let (mut app, _) = KeyboardValues::__boot();
        let event = __IceKeyPress {
            key: iced::keyboard::Key::Character("с".into()),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyC),
            location: iced::keyboard::Location::Numpad,
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: Some("с".into()),
            repeat: false,
        };
        let _ = app.__update(__KeyboardValuesMessage::Pressed(event));

        assert_eq!(
            app.dynamic_native,
            Some(iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Xkb(42)
            ))
        );
        assert_eq!(app.platform_command, iced::keyboard::Modifiers::COMMAND);
        assert_eq!(app.latin.as_deref(), Some("c"));
        assert_eq!(app.kind, "character");
        assert_eq!(app.named.as_deref(), Some("Enter"));
        assert_eq!(app.character.as_deref(), Some("с"));
        assert_eq!(app.code.as_deref(), Some("KeyC"));
        assert_eq!(app.location_name, "numpad");
        assert!(app.modifiers.control());
    }
}

#[cfg(test)]
mod pointer_values {
    ui_lang::include_app!("tests/cases/ui/pointer_values.ice");

    #[test]
    fn preserves_native_pointer_values() {
        let (mut app, _) = PointerValues::__boot();
        let _ = app.__update(__PointerValuesMessage::Inspect);

        assert_eq!(app.cursor_position, Some(iced::Point::new(12.0, 24.0)));
        assert_eq!(app.cursor_in, Some(iced::Point::new(2.0, 4.0)));
        assert!(app.cursor_levitating);
        assert!(app.over);
        assert_eq!(app.click_kind, "single");
        assert_eq!(app.width, 40.0);

        let _ = app.__update(__PointerValuesMessage::Pressed(iced::mouse::Button::Other(
            9,
        )));
        assert_eq!(app.button, iced::mouse::Button::Other(9));
        assert_eq!(app.button_kind, "other");
        assert_eq!(app.button_number, Some(9));

        let _ = app.__update(__PointerValuesMessage::Touched(
            iced::touch::Finger(u64::MAX),
            7.0,
            8.0,
        ));
        assert_eq!(app.finger, iced::touch::Finger(u64::MAX));
        assert_eq!(app.finger_id, u64::MAX.to_string());
    }
}

#[cfg(test)]
mod transformation_values {
    ui_lang::include_app!("tests/cases/ui/transformation_values.ice");

    #[test]
    fn preserves_and_applies_native_transformations() {
        let (mut app, _) = TransformationValues::__boot();
        let _ = app.__update(__TransformationValuesMessage::Inspect);

        assert_eq!(app.translation, iced::Vector::new(10.0, 20.0));
        assert_eq!(app.scale_factor, 2.0);
        assert_eq!(app.matrix.len(), 16);
        assert_eq!(app.point_value, iced::Point::new(12.0, 24.0));
        assert_eq!(app.vector_value, iced::Vector::new(2.0, 4.0));
        assert_eq!(app.size_value, iced::Size::new(6.0, 8.0));
        assert_eq!(
            app.bounds,
            iced::Rectangle {
                x: 12.0,
                y: 24.0,
                width: 6.0,
                height: 8.0,
            }
        );
        assert_eq!(app.cursor.position(), Some(iced::Point::new(12.0, 24.0)));
        assert_eq!(app.click.position(), iced::Point::new(12.0, 24.0));
        assert_eq!(app.recovered, iced::Point::new(1.0, 2.0));
        assert!(app.identity_equal);
        assert!(app.maybe_projection.is_some());
        assert!(app.invalid_projection.is_none());
    }
}

#[cfg(test)]
mod geometry_values {
    ui_lang::include_app!("tests/cases/ui/geometry_values.ice");

    #[test]
    fn preserves_and_applies_native_geometry_values() {
        let (mut app, _) = GeometryValues::__boot();
        let _ = app.__update(__GeometryValuesMessage::Inspect);

        assert_eq!(app.point_value, iced::Point::new(3.25, 4.75));
        assert_eq!(app.point_difference, iced::Vector::new(3.25, 4.75));
        assert_eq!(app.point_distance, 5.0);
        assert_eq!(app.snapped_point, iced::Point::new(3, 5));
        assert_eq!((app.snapped_x, app.snapped_y), (3, 5));
        assert_eq!(
            (app.exact_x, app.exact_y, app.exact_width, app.exact_height),
            (1, 2, 3, 4)
        );
        assert_eq!(app.point_values, [3.25, 4.75]);
        assert_eq!(app.point_display, "Point { x: 3.25, y: 4.75 }");
        assert_eq!(app.vector_value, iced::Vector::new(3.0, 3.0));
        assert_eq!(app.size_min, iced::Size::new(3.0, 2.0));
        assert_eq!(app.size_max, iced::Size::new(10.0, 8.0));
        assert_eq!(app.size_expanded, iced::Size::new(13.0, 10.0));
        assert_eq!(
            app.size_rotated,
            iced::Size::new(2.0, 4.0).rotate(iced::Radians(0.5))
        );
        assert_eq!(app.size_ratio, iced::Size::new(50.0, 50.0));
        assert_eq!(app.size_value, iced::Size::new(14.0, 27.0));
        assert_eq!(app.size_from_u32, iced::Size::new(640.0, 480.0));
        assert_eq!(app.maybe_size, Some(iced::Size::new(640.0, 480.0)));
        assert_eq!(app.invalid_size, None);
        assert_eq!(app.size_vector, iced::Vector::new(14.0, 27.0));
        assert_eq!(
            app.sized_bounds,
            iced::Rectangle::with_size(iced::Size::new(5.0, 6.0))
        );
        assert_eq!(app.radius_bounds, iced::Rectangle::with_radius(3.0));
        assert!((app.vertex_rotation - std::f64::consts::FRAC_PI_2).abs() < 0.0001);
        assert!(app.contains_point);
        assert_eq!(app.point_to_bounds, 5.0);
        assert_eq!(app.bounds_offset, iced::Vector::new(2.0, 2.0));
        assert!(app.within_bounds);
        assert_eq!(
            app.intersection,
            Some(iced::Rectangle {
                x: 5.0,
                y: 5.0,
                width: 5.0,
                height: 5.0
            })
        );
        assert!(app.intersects_bounds);
        assert_eq!(
            app.union_bounds,
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 15.0,
                height: 15.0
            }
        );
        assert_eq!(
            app.snapped_bounds,
            Some(iced::Rectangle {
                x: 1,
                y: 3,
                width: 4,
                height: 4
            })
        );
        assert_eq!(
            app.expanded_bounds,
            iced::Rectangle {
                x: 6.0,
                y: 19.0,
                width: 46.0,
                height: 64.0
            }
        );
        assert_eq!(
            app.shrunk_bounds,
            iced::Rectangle {
                x: 14.0,
                y: 21.0,
                width: 34.0,
                height: 56.0
            }
        );
        assert_eq!(
            app.rotated_bounds,
            iced::Rectangle {
                x: 10.0,
                y: 20.0,
                width: 40.0,
                height: 60.0
            }
            .rotate(iced::Radians(0.5))
        );
        assert_eq!(
            app.zoomed_bounds,
            iced::Rectangle {
                x: -10.0,
                y: -10.0,
                width: 80.0,
                height: 120.0
            }
        );
        assert_eq!(app.anchor, iced::Point::new(40.0, 60.0));
        assert_eq!(
            app.converted_bounds,
            iced::Rectangle {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        assert_eq!(
            app.moved_bounds,
            iced::Rectangle {
                x: 11.0,
                y: 22.0,
                width: 40.0,
                height: 60.0
            }
        );
        assert_eq!(
            app.scaled_bounds,
            iced::Rectangle {
                x: 20.0,
                y: 40.0,
                width: 80.0,
                height: 120.0
            }
        );
        assert_eq!(app.center, iced::Point::new(30.0, 50.0));
        assert_eq!(app.bounds_size, iced::Size::new(40.0, 60.0));
        assert_eq!(app.area, 2400.0);
    }
}

#[cfg(test)]
mod padding_angles {
    ui_lang::include_app!("tests/cases/ui/padding_angles.ice");

    #[test]
    fn preserves_native_padding_and_angle_values() {
        let (mut app, _) = PaddingAngles::__boot();
        let _ = app.__update(__PaddingAnglesMessage::Inspect);

        assert_eq!(app.pixel_value, iced::Pixels(8.0));
        assert_eq!(app.u32_pixels, iced::Pixels(u32::MAX as f32));
        assert_eq!(app.maybe_pixels, Some(iced::Pixels(42.0)));
        assert!(app.invalid_pixels.is_none());
        assert!(app.pixel_ordered);
        assert_eq!(app.all_padding, iced::Padding::new(5.0));
        assert_eq!(app.pixel_padding, iced::Padding::new(6.0));
        assert_eq!(app.top_padding, iced::Padding::ZERO.top(1.0));
        assert_eq!(app.right_padding, iced::Padding::ZERO.right(2.0));
        assert_eq!(app.bottom_padding, iced::Padding::ZERO.bottom(3.0));
        assert_eq!(app.left_padding, iced::Padding::ZERO.left(4.0));
        assert_eq!(app.horizontal_padding, iced::Padding::ZERO.horizontal(5.0));
        assert_eq!(app.vertical_padding, iced::Padding::ZERO.vertical(6.0));
        assert_eq!(app.axes_padding, iced::Padding::from([7.0, 8.0]));
        assert_eq!(
            app.changed_padding,
            iced::Padding {
                top: 6.0,
                right: 5.0,
                bottom: 6.0,
                left: 5.0
            }
        );
        assert_eq!(
            app.fitted_padding,
            iced::Padding {
                top: 3.0,
                right: 0.0,
                bottom: 0.0,
                left: 2.0
            }
        );
        assert_eq!(app.padding_size, iced::Size::new(6.0, 4.0));
        assert_eq!(
            app.expanded_bounds,
            iced::Rectangle {
                x: 6.0,
                y: 19.0,
                width: 36.0,
                height: 44.0
            }
        );
        assert_eq!(
            app.shrunk_bounds,
            iced::Rectangle {
                x: 14.0,
                y: 21.0,
                width: 24.0,
                height: 36.0
            }
        );
        assert_eq!((app.padding_x, app.padding_y), (6.0, 4.0));
        assert!(app.padding_equal);
        assert_eq!(app.degree_value, iced::Degrees(90.0));
        assert_eq!(app.degree_start, *iced::Degrees::RANGE.start());
        assert_eq!(app.degree_end, *iced::Degrees::RANGE.end());
        assert!(app.degree_in_range);
        assert!(!app.degree_out_of_range);
        assert!(app.degree_ordered);
        assert_eq!(app.radians_start, *iced::Radians::RANGE.start());
        assert_eq!(app.radians_end, *iced::Radians::RANGE.end());
        assert_eq!(app.radians_pi, iced::Radians::PI);
        assert_eq!(
            app.radians_from_degrees,
            iced::Radians::from(iced::Degrees(180.0))
        );
        assert!((app.radians_math.0 - (1.0 + std::f32::consts::PI)).abs() < 0.0001);
        assert_eq!(app.radians_reverse, iced::Radians(3.0));
        assert!(app.radians_in_range);
        assert!(app.radians_equal_scalar);
        assert_eq!(app.radians_display, "1 rad");
        assert!((app.distance_start.x - 50.0).abs() < 0.0001);
        assert!((app.distance_start.y - 50.0).abs() < 0.0001);
        assert!((app.distance_end.x - 50.0).abs() < 0.0001);
        assert!((app.distance_end.y - 0.0).abs() < 0.0001);
        assert_eq!(
            app.rotated_size,
            iced::Size::new(10.0, 20.0).rotate(iced::Radians(1.0))
        );
        assert_eq!(
            app.rotated_bounds,
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 20.0
            }
            .rotate(iced::Radians(1.0))
        );
        assert!((app.vertices_angle.0 - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }
}
