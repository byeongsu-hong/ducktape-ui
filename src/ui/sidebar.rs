//! Controlled, composable sidebar navigation.
//!
//! State stays in the application: pass [`SidebarState`] back to [`sidebar`]
//! after reducing [`SidebarAction`]s. Global shortcuts must be routed from the
//! application's iced event subscription through [`shortcut_action`]. iced
//! does not currently expose DOM-style navigation landmarks or inherited text
//! direction, so this module implements focus and activation while callers pass
//! [`Direction`] explicitly.

use super::direction::{Direction, directed_row};
use super::focus_control::{self, focus_control};
use super::popover::Placement;
use super::scroll_area::scroll_area;
use super::skeleton::skeleton;
use super::theme::{Theme, alpha, mix};
use super::tooltip::{TooltipId, tooltip};
use iced::advanced::widget;
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard;
use iced::widget::rule::{FillMode, Style as RuleStyle};
use iced::widget::text::{IntoFragment, LineHeight};
use iced::widget::{
    Column, Container, Row, Scrollable, Space, Stack, container, mouse_area, rule, text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Pixels, Shadow, Vector,
};

/// Default shadcn-compatible sidebar measurements, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarMetrics {
    pub desktop_width: f32,
    pub mobile_width: f32,
    pub icon_width: f32,
    pub floating_padding: f32,
    pub section_padding: f32,
    pub group_label_height: f32,
    pub menu_gap: f32,
    pub menu_small_height: f32,
    pub menu_default_height: f32,
    pub menu_large_height: f32,
    pub compact_control_size: f32,
    pub rail_hit_size: f32,
    pub seam_width: f32,
    pub submenu_height: f32,
}

pub const SIDEBAR_METRICS: SidebarMetrics = SidebarMetrics {
    desktop_width: 256.0,
    mobile_width: 288.0,
    icon_width: 48.0,
    floating_padding: 8.0,
    section_padding: 8.0,
    group_label_height: 32.0,
    menu_gap: 2.0,
    menu_small_height: 28.0,
    menu_default_height: 32.0,
    menu_large_height: 48.0,
    compact_control_size: 20.0,
    rail_hit_size: 16.0,
    seam_width: 1.0,
    submenu_height: 28.0,
};

impl SidebarMetrics {
    pub fn menu_height(self, size: SidebarMenuButtonSize) -> f32 {
        match size {
            SidebarMenuButtonSize::Small => self.menu_small_height,
            SidebarMenuButtonSize::Default => self.menu_default_height,
            SidebarMenuButtonSize::Large => self.menu_large_height,
        }
    }

    pub fn panel_width(
        self,
        state: SidebarState,
        viewport: SidebarViewport,
        collapsible: SidebarCollapsible,
    ) -> f32 {
        match viewport {
            SidebarViewport::Mobile => {
                if state.mobile_open {
                    self.mobile_width
                } else {
                    0.0
                }
            }
            SidebarViewport::Desktop => match collapsible {
                SidebarCollapsible::None => self.desktop_width,
                SidebarCollapsible::Offcanvas if !state.expanded => 0.0,
                SidebarCollapsible::Icon if !state.expanded => self.icon_width,
                SidebarCollapsible::Offcanvas | SidebarCollapsible::Icon => self.desktop_width,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

impl SidebarSide {
    const fn panel_alignment(self) -> Horizontal {
        match self {
            Self::Left => Horizontal::Left,
            Self::Right => Horizontal::Right,
        }
    }

    const fn tooltip_placement(self) -> Placement {
        match self {
            Self::Left => Placement::Right,
            Self::Right => Placement::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarVariant {
    #[default]
    Sidebar,
    Floating,
    Inset,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarCollapsible {
    #[default]
    Offcanvas,
    Icon,
    None,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarViewport {
    #[default]
    Desktop,
    Mobile,
}

/// Caller-owned desktop and mobile visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarState {
    pub expanded: bool,
    pub mobile_open: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            expanded: true,
            mobile_open: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Toggle(SidebarViewport),
    SetExpanded(bool),
    SetMobileOpen(bool),
    CloseMobile,
}

impl SidebarState {
    pub const fn is_open(self, viewport: SidebarViewport) -> bool {
        match viewport {
            SidebarViewport::Desktop => self.expanded,
            SidebarViewport::Mobile => self.mobile_open,
        }
    }

    pub const fn is_collapsed(
        self,
        viewport: SidebarViewport,
        collapsible: SidebarCollapsible,
    ) -> bool {
        match viewport {
            SidebarViewport::Mobile => !self.mobile_open,
            SidebarViewport::Desktop => {
                !self.expanded && !matches!(collapsible, SidebarCollapsible::None)
            }
        }
    }

    #[must_use]
    pub const fn reduced(self, action: SidebarAction) -> Self {
        match action {
            SidebarAction::Toggle(SidebarViewport::Desktop) => Self {
                expanded: !self.expanded,
                ..self
            },
            SidebarAction::Toggle(SidebarViewport::Mobile) => Self {
                mobile_open: !self.mobile_open,
                ..self
            },
            SidebarAction::SetExpanded(expanded) => Self { expanded, ..self },
            SidebarAction::SetMobileOpen(mobile_open) => Self {
                mobile_open,
                ..self
            },
            SidebarAction::CloseMobile => Self {
                mobile_open: false,
                ..self
            },
        }
    }
}

/// Maps Ctrl+B or Command+B to the controlled toggle action.
///
/// Shift/Alt modified chords are left to the application. Both Ctrl and Logo
/// are accepted on every platform so remote keyboards behave consistently.
pub fn shortcut_action(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    viewport: SidebarViewport,
) -> Option<SidebarAction> {
    let is_b = matches!(key, keyboard::Key::Character(value) if value.eq_ignore_ascii_case("b"));
    let primary = modifiers.control() || modifiers.logo();
    (is_b && primary && !modifiers.alt() && !modifiers.shift())
        .then_some(SidebarAction::Toggle(viewport))
}

/// Stable IDs for the sidebar rail and its related controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarId(String);

impl SidebarId {
    pub fn new(key: impl ToString) -> Self {
        Self(key.to_string())
    }

    fn rail(&self) -> widget::Id {
        widget::Id::from(format!("ducktape-sidebar:{}:rail", self.0))
    }
}

/// A controlled sidebar surface with sticky header/footer and scrolling body.
pub struct Sidebar<'a, Message>
where
    Message: Clone + 'a,
{
    id: SidebarId,
    state: SidebarState,
    on_toggle: Message,
    header: Option<Element<'a, Message>>,
    content: Element<'a, Message>,
    footer: Option<Element<'a, Message>>,
    side: SidebarSide,
    variant: SidebarVariant,
    collapsible: SidebarCollapsible,
    viewport: SidebarViewport,
    rail: bool,
    metrics: SidebarMetrics,
    theme: Theme,
}

pub fn sidebar<'a, Message>(
    id: SidebarId,
    state: SidebarState,
    content: impl Into<Element<'a, Message>>,
    on_toggle: Message,
    theme: &Theme,
) -> Sidebar<'a, Message>
where
    Message: Clone + 'a,
{
    Sidebar {
        id,
        state,
        on_toggle,
        header: None,
        content: content.into(),
        footer: None,
        side: SidebarSide::Left,
        variant: SidebarVariant::Sidebar,
        collapsible: SidebarCollapsible::Offcanvas,
        viewport: SidebarViewport::Desktop,
        rail: true,
        metrics: SIDEBAR_METRICS,
        theme: *theme,
    }
}

impl<'a, Message> Sidebar<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    #[must_use]
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: SidebarVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn collapsible(mut self, collapsible: SidebarCollapsible) -> Self {
        self.collapsible = collapsible;
        self
    }

    #[must_use]
    pub fn viewport(mut self, viewport: SidebarViewport) -> Self {
        self.viewport = viewport;
        self
    }

    #[must_use]
    pub fn rail(mut self, rail: bool) -> Self {
        self.rail = rail;
        self
    }

    #[must_use]
    pub fn metrics(mut self, metrics: SidebarMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn into_widget(self) -> Element<'a, Message> {
        let width = self
            .metrics
            .panel_width(self.state, self.viewport, self.collapsible);
        if width == 0.0 {
            return Space::new().width(0).height(Length::Fill).into();
        }

        let mut sections = Column::new().width(Length::Fill).height(Length::Fill);
        if let Some(header) = self.header {
            sections = sections.push(header);
        }
        sections = sections.push(sidebar_content(self.content, &self.theme));
        if let Some(footer) = self.footer {
            sections = sections.push(footer);
        }

        let panel_theme = self.theme;
        let panel = container(sections)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_iced_theme| panel_style(&panel_theme, self.variant));
        let panel: Element<'a, Message> = match self.variant {
            SidebarVariant::Sidebar => panel.into(),
            SidebarVariant::Floating | SidebarVariant::Inset => container(panel)
                .padding(self.metrics.floating_padding)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        };

        let mut layers = Stack::new()
            .width(width)
            .height(Length::Fill)
            .clip(true)
            .push(panel);
        if self.variant == SidebarVariant::Sidebar {
            layers = layers.push(sidebar_seam(
                self.side,
                self.metrics.seam_width,
                &self.theme,
            ));
        }
        if self.rail && self.collapsible != SidebarCollapsible::None {
            let rail = sidebar_rail(
                self.id.rail(),
                self.on_toggle,
                self.side,
                self.metrics,
                &self.theme,
            );
            layers = layers.push(
                container(rail)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(self.side.panel_alignment()),
            );
        }

        layers.into()
    }
}

impl<'a, Message> From<Sidebar<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(sidebar: Sidebar<'a, Message>) -> Self {
        sidebar.into_widget()
    }
}

/// Places the panel beside desktop content or over mobile content.
///
/// The mobile backdrop closes the controlled sheet. No animation is imposed,
/// so reduced-motion behavior is deterministic.
pub fn sidebar_layout<'a, Message>(
    main: impl Into<Element<'a, Message>>,
    panel: impl Into<Element<'a, Message>>,
    state: SidebarState,
    viewport: SidebarViewport,
    side: SidebarSide,
    on_close_mobile: Message,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let main = main.into();
    let panel = panel.into();
    match viewport {
        SidebarViewport::Desktop => {
            let mut row = Row::new().width(Length::Fill).height(Length::Fill);
            if side == SidebarSide::Left {
                row = row.push(panel).push(main);
            } else {
                row = row.push(main).push(panel);
            }
            row.into()
        }
        SidebarViewport::Mobile if state.mobile_open => {
            let backdrop_theme = *theme;
            let backdrop = mouse_area(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_iced_theme| backdrop_style(&backdrop_theme)),
            )
            .on_press(on_close_mobile);
            let aligned_panel = container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(side.panel_alignment());
            Stack::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(main)
                .push(backdrop)
                .push(aligned_panel)
                .into()
        }
        SidebarViewport::Mobile => main,
    }
}

pub fn sidebar_header<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content)
        .padding(SIDEBAR_METRICS.section_padding)
        .width(Length::Fill)
}

pub fn sidebar_content<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Scrollable<'a, Message>
where
    Message: 'a,
{
    scroll_area(content, theme)
        .width(Length::Fill)
        .height(Length::Fill)
}

pub fn sidebar_footer<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content)
        .padding(SIDEBAR_METRICS.section_padding)
        .width(Length::Fill)
}

pub fn sidebar_group<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content)
        .padding(SIDEBAR_METRICS.section_padding)
        .width(Length::Fill)
}

/// A 32px group label. Icon-collapsed sidebars omit it entirely.
pub fn sidebar_group_label<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    collapsed: bool,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    if collapsed {
        return Space::new().height(0).width(Length::Fill).into();
    }
    container(content)
        .padding([0.0, SIDEBAR_METRICS.section_padding])
        .width(Length::Fill)
        .height(SIDEBAR_METRICS.group_label_height)
        .align_x(direction.start())
        .align_y(Vertical::Center)
        .style({
            let color = theme.palette.muted_foreground;
            move |_iced_theme| iced::widget::container::Style {
                text_color: Some(color),
                ..Default::default()
            }
        })
        .into()
}

/// Composes a label and its optional 20px action in reading order.
pub fn sidebar_group_heading<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    action: Option<Element<'a, Message>>,
    direction: Direction,
) -> Row<'a, Message>
where
    Message: 'a,
{
    let mut items = vec![container(label).width(Length::Fill).into()];
    if let Some(action) = action {
        items.push(action);
    }
    directed_row(items, direction)
        .width(Length::Fill)
        .height(SIDEBAR_METRICS.group_label_height)
        .align_y(Alignment::Center)
}

pub fn sidebar_group_action<'a, Message>(
    id: impl Into<widget::Id>,
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    disabled: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    compact_control(id.into(), content.into(), on_press, disabled, theme)
}

pub fn sidebar_group_content<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content).width(Length::Fill)
}

pub fn sidebar_menu<'a, Message>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> Column<'a, Message>
where
    Message: 'a,
{
    Column::with_children(items)
        .spacing(SIDEBAR_METRICS.menu_gap)
        .width(Length::Fill)
}

/// Layers an action or badge over the reading-end of a full-width menu row.
pub fn sidebar_menu_item<'a, Message>(
    button: impl Into<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    direction: Direction,
) -> Stack<'a, Message>
where
    Message: 'a,
{
    let mut item = Stack::new().width(Length::Fill).push(button);
    if let Some(trailing) = trailing {
        item = item.push(
            container(trailing)
                .padding(Padding::ZERO.right(4.0))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(direction.end())
                .align_y(Vertical::Center),
        );
    }
    item
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarMenuButtonId(String);

impl SidebarMenuButtonId {
    pub fn new(key: impl ToString) -> Self {
        Self(key.to_string())
    }

    fn control(&self) -> widget::Id {
        widget::Id::from(format!("ducktape-sidebar-menu:{}:button", self.0))
    }

    fn tooltip(&self) -> TooltipId {
        TooltipId::new(format!("sidebar-menu:{}", self.0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarMenuButtonSize {
    Small,
    #[default]
    Default,
    Large,
}

/// A controlled active/disabled menu button with optional collapsed tooltip.
pub struct SidebarMenuButton<'a, Message>
where
    Message: Clone + 'a,
{
    id: SidebarMenuButtonId,
    content: Element<'a, Message>,
    on_press: Message,
    active: bool,
    disabled: bool,
    collapsed: bool,
    tooltip: Option<Element<'a, Message>>,
    side: SidebarSide,
    direction: Direction,
    size: SidebarMenuButtonSize,
    theme: Theme,
}

pub fn sidebar_menu_button<'a, Message>(
    id: SidebarMenuButtonId,
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    theme: &Theme,
) -> SidebarMenuButton<'a, Message>
where
    Message: Clone + 'a,
{
    SidebarMenuButton {
        id,
        content: content.into(),
        on_press,
        active: false,
        disabled: false,
        collapsed: false,
        tooltip: None,
        side: SidebarSide::Left,
        direction: Direction::LeftToRight,
        size: SidebarMenuButtonSize::Default,
        theme: *theme,
    }
}

impl<'a, Message> SidebarMenuButton<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    #[must_use]
    pub fn tooltip(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.tooltip = Some(content.into());
        self
    }

    #[must_use]
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn size(mut self, size: SidebarMenuButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn into_widget(self) -> Element<'a, Message> {
        let height = SIDEBAR_METRICS.menu_height(self.size);
        let theme = self.theme;
        let active = self.active;
        let content = container(self.content)
            .padding([0.0, SIDEBAR_METRICS.section_padding])
            .width(Length::Fill)
            .height(height)
            .align_y(Vertical::Center)
            .align_x(if self.collapsed {
                Horizontal::Center
            } else {
                self.direction.start()
            });
        let control: Element<'a, Message> =
            focus_control(self.id.control(), content, self.on_press, &self.theme)
                .disabled(self.disabled)
                .style(move |_iced_theme, status| menu_button_style(&theme, active, status))
                .into();

        if tooltip_enabled(self.collapsed, self.disabled, self.tooltip.is_some()) {
            // The shared tooltip supplies delayed hover/focus presentation. The
            // nested button remains the activation owner until iced gains a
            // semantic control+description relationship primitive.
            tooltip(
                self.id.tooltip(),
                control,
                self.tooltip.expect("checked above"),
                &self.theme,
            )
            .placement(self.side.tooltip_placement())
            .side_offset(8.0)
            .into()
        } else {
            control
        }
    }
}

impl<'a, Message> From<SidebarMenuButton<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(button: SidebarMenuButton<'a, Message>) -> Self {
        button.into_widget()
    }
}

/// Builds icon/label/trailing content and removes copy in icon-collapse mode.
pub fn sidebar_menu_button_content<'a, Message>(
    leading: Option<Element<'a, Message>>,
    label: impl IntoFragment<'a>,
    trailing: Option<Element<'a, Message>>,
    collapsed: bool,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let mut items = Vec::new();
    if let Some(leading) = leading {
        items.push(leading);
    }
    if !collapsed {
        items.push(
            container(
                text(label)
                    .size(theme.typography.sm)
                    .line_height(LineHeight::Absolute(Pixels(16.0))),
            )
            .width(Length::Fill)
            .into(),
        );
        if let Some(trailing) = trailing {
            items.push(trailing);
        }
    }
    directed_row(items, direction)
        .spacing(SIDEBAR_METRICS.section_padding)
        .align_y(Alignment::Center)
        .width(if collapsed {
            Length::Shrink
        } else {
            Length::Fill
        })
        .into()
}

pub fn sidebar_menu_action<'a, Message>(
    id: impl Into<widget::Id>,
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    disabled: bool,
    visible: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if !visible {
        return Space::new().width(0).height(0).into();
    }
    compact_control(id.into(), content.into(), on_press, disabled, theme)
}

pub fn sidebar_menu_badge<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    active: bool,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let style_theme = *theme;
    container(content)
        .padding([0, 6])
        .height(SIDEBAR_METRICS.compact_control_size)
        .align_y(Vertical::Center)
        .style(move |_iced_theme| badge_style(&style_theme, active))
}

pub fn sidebar_menu_skeleton<'a, Message>(
    show_icon: bool,
    collapsed: bool,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut row = Row::new()
        .spacing(SIDEBAR_METRICS.section_padding)
        .align_y(Alignment::Center);
    if show_icon {
        row = row.push(skeleton(theme).width(16).height(16));
    }
    if !collapsed {
        row = row.push(skeleton(theme).width(Length::Fill).height(12));
    }
    container(row)
        .padding([0.0, SIDEBAR_METRICS.section_padding])
        .width(Length::Fill)
        .height(SIDEBAR_METRICS.menu_default_height)
        .align_y(Vertical::Center)
}

/// Indented submenu with a reading-start seam.
pub fn sidebar_submenu<'a, Message>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
    direction: Direction,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    let seam: Element<'a, Message> = rule::vertical::<iced::Theme>(1)
        .style(move |_theme| RuleStyle {
            color,
            radius: 0.0.into(),
            fill_mode: FillMode::Full,
            snap: true,
        })
        .into();
    let list: Element<'a, Message> = Column::with_children(items)
        .spacing(SIDEBAR_METRICS.menu_gap)
        .width(Length::Fill)
        .into();
    container(
        directed_row(vec![seam, list], direction)
            .spacing(10.0)
            .width(Length::Fill),
    )
    .padding([2, 12])
    .width(Length::Fill)
}

pub fn sidebar_submenu_item<'a, Message>(
    button: impl Into<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    direction: Direction,
) -> Stack<'a, Message>
where
    Message: 'a,
{
    sidebar_menu_item(button, trailing, direction)
}

pub fn sidebar_submenu_button<'a, Message>(
    id: SidebarMenuButtonId,
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    theme: &Theme,
) -> SidebarMenuButton<'a, Message>
where
    Message: Clone + 'a,
{
    sidebar_menu_button(id, content, on_press, theme).size(SidebarMenuButtonSize::Small)
}

fn compact_control<'a, Message>(
    id: widget::Id,
    content: Element<'a, Message>,
    on_press: Message,
    disabled: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let content = container(content)
        .width(SIDEBAR_METRICS.compact_control_size)
        .height(SIDEBAR_METRICS.compact_control_size)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);
    let style_theme = *theme;
    focus_control(id, content, on_press, theme)
        .disabled(disabled)
        .style(move |_iced_theme, status| compact_control_style(&style_theme, status))
        .into()
}

fn sidebar_rail<'a, Message>(
    id: widget::Id,
    on_toggle: Message,
    side: SidebarSide,
    metrics: SidebarMetrics,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let color = theme.palette.border;
    let seam: Element<'a, Message> = rule::vertical::<iced::Theme>(metrics.seam_width)
        .style(move |_theme| RuleStyle {
            color,
            radius: 0.0.into(),
            fill_mode: FillMode::Full,
            snap: true,
        })
        .into();
    let content = container(seam)
        .width(metrics.rail_hit_size)
        .height(Length::Fill)
        .align_x(Horizontal::Center);
    let style_theme = *theme;
    focus_control(id, content, on_toggle, theme)
        .style(move |_iced_theme, status| rail_style(&style_theme, side, status))
        .into()
}

fn sidebar_seam<'a, Message>(side: SidebarSide, width: f32, theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    container(
        rule::vertical::<iced::Theme>(width).style(move |_theme| RuleStyle {
            color,
            radius: 0.0.into(),
            fill_mode: FillMode::Full,
            snap: true,
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(side.panel_alignment())
}

fn tooltip_enabled(collapsed: bool, disabled: bool, has_tooltip: bool) -> bool {
    collapsed && !disabled && has_tooltip
}

pub fn panel_style(theme: &Theme, variant: SidebarVariant) -> iced::widget::container::Style {
    let floating = variant == SidebarVariant::Floating;
    iced::widget::container::Style {
        background: Some(Background::Color(match variant {
            SidebarVariant::Sidebar => theme.palette.muted,
            SidebarVariant::Floating | SidebarVariant::Inset => theme.palette.card,
        })),
        text_color: Some(theme.palette.foreground),
        border: Border {
            color: theme.palette.border,
            width: if floating { 1.0 } else { 0.0 },
            radius: match variant {
                SidebarVariant::Sidebar => 0.0,
                SidebarVariant::Floating => theme.radius.lg,
                SidebarVariant::Inset => theme.radius.md,
            }
            .into(),
        },
        shadow: if floating {
            Shadow {
                color: alpha(Color::BLACK, 0.12),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}

fn backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(alpha(theme.palette.foreground, 0.42))),
        ..Default::default()
    }
}

pub fn menu_button_style(
    theme: &Theme,
    active: bool,
    status: focus_control::Status,
) -> focus_control::Style {
    let disabled = status == focus_control::Status::Disabled;
    let hovered = matches!(
        status,
        focus_control::Status::Hovered | focus_control::Status::Pressed
    );
    let background = if active || hovered {
        Some(Background::Color(
            if status == focus_control::Status::Pressed {
                mix(theme.palette.accent, theme.palette.foreground, 0.08)
            } else {
                theme.palette.accent
            },
        ))
    } else {
        None
    };
    let foreground = if active || hovered {
        theme.palette.accent_foreground
    } else {
        theme.palette.foreground
    };

    focus_control::Style {
        background,
        text_color: Some(if disabled {
            alpha(foreground, 0.5)
        } else {
            foreground
        }),
        border: Border {
            radius: theme.radius.sm.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: if disabled { 0.0 } else { 2.0 },
            radius: (theme.radius.sm + 2.0).into(),
        },
        focus_offset: 0.0,
    }
}

fn compact_control_style(theme: &Theme, status: focus_control::Status) -> focus_control::Style {
    let disabled = status == focus_control::Status::Disabled;
    let hovered = matches!(
        status,
        focus_control::Status::Hovered | focus_control::Status::Pressed
    );
    focus_control::Style {
        background: hovered.then_some(Background::Color(theme.palette.accent)),
        text_color: Some(if disabled {
            alpha(theme.palette.foreground, 0.5)
        } else if hovered {
            theme.palette.accent_foreground
        } else {
            theme.palette.foreground
        }),
        border: Border {
            radius: theme.radius.sm.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: if disabled { 0.0 } else { 2.0 },
            radius: theme.radius.sm.into(),
        },
        focus_offset: 0.0,
    }
}

fn rail_style(
    theme: &Theme,
    _side: SidebarSide,
    status: focus_control::Status,
) -> focus_control::Style {
    let hovered = matches!(
        status,
        focus_control::Status::Hovered | focus_control::Status::Pressed
    );
    focus_control::Style {
        background: hovered.then_some(Background::Color(alpha(theme.palette.accent, 0.55))),
        text_color: None,
        border: Border::default(),
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: 0.0.into(),
        },
        focus_offset: 0.0,
    }
}

fn badge_style(theme: &Theme, active: bool) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: active.then_some(Background::Color(theme.palette.accent)),
        text_color: Some(if active {
            theme.palette.accent_foreground
        } else {
            theme.palette.muted_foreground
        }),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    #[test]
    fn reducer_keeps_desktop_and_mobile_state_independent() {
        let state = SidebarState::default()
            .reduced(SidebarAction::Toggle(SidebarViewport::Desktop))
            .reduced(SidebarAction::Toggle(SidebarViewport::Mobile));
        assert_eq!(
            state,
            SidebarState {
                expanded: false,
                mobile_open: true,
            }
        );
        assert_eq!(
            state.reduced(SidebarAction::CloseMobile),
            SidebarState {
                expanded: false,
                mobile_open: false,
            }
        );
    }

    #[test]
    fn shortcut_accepts_ctrl_or_command_b_without_extra_modifiers() {
        let b = keyboard::Key::Character("b".into());
        let capital_b = keyboard::Key::Character("B".into());
        let desktop = SidebarAction::Toggle(SidebarViewport::Desktop);
        assert_eq!(
            shortcut_action(&b, keyboard::Modifiers::CTRL, SidebarViewport::Desktop),
            Some(desktop)
        );
        assert_eq!(
            shortcut_action(
                &capital_b,
                keyboard::Modifiers::LOGO,
                SidebarViewport::Desktop
            ),
            Some(desktop)
        );
        assert_eq!(
            shortcut_action(
                &b,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                SidebarViewport::Desktop
            ),
            None
        );
        assert_eq!(
            shortcut_action(
                &keyboard::Key::Character("x".into()),
                keyboard::Modifiers::CTRL,
                SidebarViewport::Desktop
            ),
            None
        );
    }

    #[test]
    fn widths_cover_every_collapsible_and_viewport_mode() {
        let expanded = SidebarState::default();
        let collapsed = expanded.reduced(SidebarAction::SetExpanded(false));
        assert_eq!(
            SIDEBAR_METRICS.panel_width(
                expanded,
                SidebarViewport::Desktop,
                SidebarCollapsible::Icon
            ),
            256.0
        );
        assert_eq!(
            SIDEBAR_METRICS.panel_width(
                collapsed,
                SidebarViewport::Desktop,
                SidebarCollapsible::Icon
            ),
            48.0
        );
        assert_eq!(
            SIDEBAR_METRICS.panel_width(
                collapsed,
                SidebarViewport::Desktop,
                SidebarCollapsible::Offcanvas
            ),
            0.0
        );
        assert_eq!(
            SIDEBAR_METRICS.panel_width(
                collapsed,
                SidebarViewport::Desktop,
                SidebarCollapsible::None
            ),
            256.0
        );
        assert_eq!(
            SIDEBAR_METRICS.panel_width(
                SidebarState {
                    mobile_open: true,
                    ..collapsed
                },
                SidebarViewport::Mobile,
                SidebarCollapsible::Icon
            ),
            288.0
        );
    }

    #[test]
    fn metrics_lock_row_heights_padding_and_seams() {
        assert_eq!(SIDEBAR_METRICS.section_padding, 8.0);
        assert_eq!(SIDEBAR_METRICS.group_label_height, 32.0);
        assert_eq!(
            SIDEBAR_METRICS.menu_height(SidebarMenuButtonSize::Small),
            28.0
        );
        assert_eq!(
            SIDEBAR_METRICS.menu_height(SidebarMenuButtonSize::Default),
            32.0
        );
        assert_eq!(
            SIDEBAR_METRICS.menu_height(SidebarMenuButtonSize::Large),
            48.0
        );
        assert_eq!(SIDEBAR_METRICS.compact_control_size, 20.0);
        assert_eq!(SIDEBAR_METRICS.rail_hit_size, 16.0);
        assert_eq!(SIDEBAR_METRICS.seam_width, 1.0);
    }

    #[test]
    fn physical_side_and_reading_direction_are_explicit() {
        assert_eq!(SidebarSide::Left.panel_alignment(), Horizontal::Left);
        assert_eq!(SidebarSide::Right.panel_alignment(), Horizontal::Right);
        assert_eq!(SidebarSide::Left.tooltip_placement(), Placement::Right);
        assert_eq!(SidebarSide::Right.tooltip_placement(), Placement::Left);
        assert_eq!(Direction::LeftToRight.start(), Horizontal::Left);
        assert_eq!(Direction::RightToLeft.start(), Horizontal::Right);
    }

    #[test]
    fn variants_use_expected_surfaces_borders_and_shadow() {
        let plain = panel_style(&LIGHT, SidebarVariant::Sidebar);
        let floating = panel_style(&LIGHT, SidebarVariant::Floating);
        let inset = panel_style(&DARK, SidebarVariant::Inset);
        assert_eq!(
            plain.background,
            Some(Background::Color(LIGHT.palette.muted))
        );
        assert_eq!(plain.border.width, 0.0);
        assert_eq!(floating.border.width, 1.0);
        assert!(floating.shadow.color.a > 0.0);
        assert_eq!(inset.background, Some(Background::Color(DARK.palette.card)));
        assert_eq!(inset.border.radius, DARK.radius.md.into());
    }

    #[test]
    fn menu_states_follow_semantic_light_and_dark_colors() {
        for theme in [LIGHT, DARK] {
            let active = menu_button_style(&theme, true, focus_control::Status::Active);
            let hovered = menu_button_style(&theme, false, focus_control::Status::Hovered);
            let focused = menu_button_style(&theme, false, focus_control::Status::Focused);
            let disabled = menu_button_style(&theme, false, focus_control::Status::Disabled);
            assert_eq!(
                active.background,
                Some(Background::Color(theme.palette.accent))
            );
            assert_eq!(
                hovered.background,
                Some(Background::Color(theme.palette.accent))
            );
            assert_eq!(focused.focus_ring.color, theme.palette.ring);
            assert_eq!(focused.focus_ring.width, 2.0);
            assert_eq!(disabled.focus_ring.width, 0.0);
            assert_eq!(
                disabled.text_color,
                Some(alpha(theme.palette.foreground, 0.5))
            );
        }
    }

    #[test]
    fn collapsed_tooltip_contract_excludes_expanded_and_disabled_items() {
        assert!(tooltip_enabled(true, false, true));
        assert!(!tooltip_enabled(false, false, true));
        assert!(!tooltip_enabled(true, true, true));
        assert!(!tooltip_enabled(true, false, false));
    }
}
