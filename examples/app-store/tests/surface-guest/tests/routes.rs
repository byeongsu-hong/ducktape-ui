use app_store_surface_fixture::{boot_native, tick_native};
use ui_lang_guest::testing::has_text;
use ui_lang_guest::wire::{Event, Frame, Node, SurfaceValue as V};

fn surface(frame: &Frame, name: &str, position: usize) -> (Vec<V>, Option<u32>) {
    fn walk<'a>(node: &'a Node, name: &str, out: &mut Vec<&'a Node>) {
        if matches!(node, Node::Surface { name: found, .. } if found == name) {
            out.push(node);
        }
        for child in node.children() {
            walk(child, name, out);
        }
    }
    let mut nodes = vec![];
    walk(frame.root.as_ref().unwrap(), name, &mut nodes);
    let Node::Surface { args, on_event, .. } = nodes[position] else {
        unreachable!()
    };
    (args.clone(), *on_event)
}

#[test]
fn surface_values_return_to_generated_handlers_with_owned_row_context() {
    boot_native();
    let frame = tick_native(vec![]);
    let (args, route) = surface(&frame, "preview", 1);
    assert_eq!(
        args,
        [
            V::Str("second".into()),
            V::Bool(false),
            V::I64(7),
            V::F64(1.5)
        ]
    );
    assert_eq!(surface(&frame, "quiet", 0).1, None);
    let wrong = tick_native(vec![Event::Surface {
        handler: route.unwrap(),
        value: V::Bool(true),
    }]);
    assert!(has_text(&wrong, "initial"));
    let route = surface(&wrong, "preview", 1).1.unwrap();
    let linked = tick_native(vec![Event::Surface {
        handler: route,
        value: V::Str("duck://pages/example".into()),
    }]);
    assert!(has_text(&linked, "duck://pages/example"));
    assert!(has_text(&linked, "second"));
    let events = [
        ("toggle", V::Bool(true)),
        ("integer", V::I64(i64::MAX)),
        ("number", V::F64(2.5)),
        ("action", V::Unit),
    ]
    .into_iter()
    .map(|(name, value)| Event::Surface {
        handler: surface(&linked, name, 0).1.unwrap(),
        value,
    })
    .collect();
    let changed = tick_native(events);
    assert_eq!(
        surface(&changed, "preview", 0).0,
        [
            V::Str("first".into()),
            V::Bool(true),
            V::I64(i64::MAX),
            V::F64(2.5)
        ]
    );
    assert!(has_text(&changed, "1"));
    let unchanged = tick_native(vec![Event::Surface {
        handler: surface(&changed, "number", 0).1.unwrap(),
        value: V::F64(f64::NAN),
    }]);
    assert_eq!(surface(&unchanged, "number", 0).0, [V::F64(2.5)]);
}
