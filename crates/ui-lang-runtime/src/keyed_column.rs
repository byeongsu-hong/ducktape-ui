//! Keyed row reconciliation with native column layout and interaction.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::Column;
use iced::{Alignment, Element, Event, Length, Padding, Pixels, Rectangle, Size, Vector};

/// A native column whose row state follows arbitrary key permutations.
pub struct KeyedColumn<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    keys: Vec<K>,
    column: Column<'a, Message, Theme, Renderer>,
}

/// Creates a keyed column. Repeated keys match their old occurrences in order.
/// Keys retain `PartialEq` semantics, including numeric float equality.
pub fn keyed_column<'a, K, Message, Theme, Renderer>(
    rows: impl IntoIterator<Item = (K, Element<'a, Message, Theme, Renderer>)>,
) -> KeyedColumn<'a, K, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    let (keys, children): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
    let mut size = Size::new(Length::Shrink, Length::Shrink);
    for child in &children {
        let hint = child.as_widget().size_hint();
        size.width = size.width.enclose(hint.width);
        size.height = size.height.enclose(hint.height);
    }
    KeyedColumn {
        keys,
        column: Column::from_vec(children)
            .width(size.width)
            .height(size.height),
    }
}

impl<K, Message, Theme, Renderer> KeyedColumn<'_, K, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Sets row spacing.
    pub fn spacing(mut self, value: impl Into<Pixels>) -> Self {
        self.column = self.column.spacing(value);
        self
    }
    /// Sets column padding.
    pub fn padding(mut self, value: impl Into<Padding>) -> Self {
        self.column = self.column.padding(value);
        self
    }
    /// Sets column width.
    pub fn width(mut self, value: impl Into<Length>) -> Self {
        self.column = self.column.width(value);
        self
    }
    /// Sets column height.
    pub fn height(mut self, value: impl Into<Length>) -> Self {
        self.column = self.column.height(value);
        self
    }
    /// Sets maximum width.
    pub fn max_width(mut self, value: impl Into<Pixels>) -> Self {
        self.column = self.column.max_width(value);
        self
    }
    /// Sets horizontal row alignment.
    pub fn align_items(mut self, value: Alignment) -> Self {
        self.column = self.column.align_x(value);
        self
    }
}

impl<K: Copy + PartialEq + 'static, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for KeyedColumn<'_, K, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Vec<K>>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(self.keys.clone())
    }
    fn children(&self) -> Vec<Tree> {
        self.column.children()
    }
    fn diff(&self, tree: &mut Tree) {
        let previous = tree.state.downcast_mut::<Vec<K>>();
        if previous != &self.keys {
            let mut rows: Vec<_> = std::mem::take(previous)
                .into_iter()
                .zip(std::mem::take(&mut tree.children))
                .map(Some)
                .collect();
            tree.children = self
                .keys
                .iter()
                .map(|key| {
                    rows.iter()
                        .position(|row| row.as_ref().is_some_and(|(old, _)| old == key))
                        .and_then(|index| rows[index].take())
                        .map_or_else(Tree::empty, |(_, tree)| tree)
                })
                .collect();
            previous.clone_from(&self.keys);
        }
        self.column.diff(tree);
    }
    fn size(&self) -> Size<Length> {
        self.column.size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.column.size_hint()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.column.layout(tree, renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.column.operate(tree, layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.column.update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.column
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.column
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.column
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<
    'a,
    K: Copy + PartialEq + 'static,
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
> From<KeyedColumn<'a, K, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
{
    fn from(column: KeyedColumn<'a, K, Message, Theme, Renderer>) -> Self {
        Self::new(column)
    }
}

#[cfg(test)]
mod tests {
    use super::keyed_column;
    use iced::advanced::widget::{Tree, tree};
    use iced::advanced::{Layout, Widget, layout, mouse, renderer};
    use iced::{Element, Length, Rectangle, Size};

    struct Stamp(i64);
    impl Widget<(), iced::Theme, iced_test::renderer::Renderer> for Stamp {
        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<i64>()
        }
        fn state(&self) -> tree::State {
            tree::State::new(self.0)
        }
        fn size(&self) -> Size<Length> {
            Size::new(Length::Shrink, Length::Shrink)
        }
        fn layout(
            &mut self,
            _: &mut Tree,
            _: &iced_test::renderer::Renderer,
            _: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(10.0, 10.0))
        }
        fn draw(
            &self,
            _: &Tree,
            _: &mut iced_test::renderer::Renderer,
            _: &iced::Theme,
            _: &renderer::Style,
            _: Layout<'_>,
            _: mouse::Cursor,
            _: &Rectangle,
        ) {
        }
    }

    #[test]
    fn rotations_keep_each_occurrences_widget_state() {
        let column = |keys: &[(i64, i64)]| {
            keyed_column(
                keys.iter()
                    .map(|(key, stamp)| (*key, Element::new(Stamp(*stamp)))),
            )
        };
        let initial = column(&[(1, 10), (2, 20), (1, 30), (3, 40)]);
        let mut tree =
            Tree::new(&initial as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        column(&[(3, 400), (1, 100), (2, 200), (1, 300)]).diff(&mut tree);
        let stamps = || {
            tree.children
                .iter()
                .map(|row| *row.state.downcast_ref::<i64>())
                .collect::<Vec<_>>()
        };
        assert_eq!(stamps(), vec![40, 10, 20, 30]);
        column(&[(0, 50), (1, 100), (3, 400)]).diff(&mut tree);
        assert_eq!(
            tree.children
                .iter()
                .map(|row| *row.state.downcast_ref::<i64>())
                .collect::<Vec<_>>(),
            vec![50, 10, 40]
        );
    }
    #[test]
    fn float_keys_keep_numeric_zero_equality_and_replace_nan() {
        let column = |rows: &[(f64, i64)]| {
            keyed_column(
                rows.iter()
                    .map(|(key, stamp)| (*key, Element::new(Stamp(*stamp)))),
            )
        };
        let initial = column(&[(-0.0, 10), (f64::NAN, 20), (1.0, 30)]);
        let mut tree =
            Tree::new(&initial as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        column(&[(1.0, 300), (0.0, 100), (f64::NAN, 200)]).diff(&mut tree);
        assert_eq!(
            tree.children
                .iter()
                .map(|row| *row.state.downcast_ref::<i64>())
                .collect::<Vec<_>>(),
            vec![30, 10, 200]
        );
    }
}
