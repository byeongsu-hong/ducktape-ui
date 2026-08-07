use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedLinearAxis {
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedFlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedFlexContentAlignment {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLinearLayout {
    pub(crate) axis: ResolvedLinearAxis,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) padding: ResolvedContainerPadding,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) max_width: Option<CheckedExprUseId>,
    /// Estimated row height; `Some` makes this column lay out only the rows
    /// the viewport can see.
    pub(crate) virtual_row: Option<CheckedExprUseId>,
    pub(crate) align: Option<ResolvedContainerAlignment>,
    pub(crate) clip: Option<CheckedExprUseId>,
    pub(crate) wrap: bool,
    pub(crate) wrap_spacing: Option<CheckedExprUseId>,
    pub(crate) wrap_align: Option<ResolvedContainerAlignment>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedGridHeight {
    AspectRatio {
        width: CheckedExprUseId,
        height: CheckedExprUseId,
    },
    EvenlyDistribute(ResolvedContainerLength),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedGridLayout {
    pub(crate) columns: Option<CheckedExprUseId>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) width: Option<CheckedExprUseId>,
    pub(crate) height: Option<ResolvedGridHeight>,
    pub(crate) max_cell: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedStackLayout {
    pub(crate) under: u16,
    pub(crate) clip: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedFlexLayout {
    pub(crate) direction: ResolvedFlexDirection,
    pub(crate) wrap: ResolvedFlexWrap,
    pub(crate) justify_content: Option<ResolvedFlexContentAlignment>,
    pub(crate) align_items: Option<ResolvedContainerFlexAlignment>,
    pub(crate) align_content: Option<ResolvedFlexContentAlignment>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) wrap_spacing: Option<CheckedExprUseId>,
    pub(crate) row_gap: Option<CheckedExprUseId>,
    pub(crate) column_gap: Option<CheckedExprUseId>,
    pub(crate) padding: ResolvedContainerPadding,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) max_width: Option<CheckedExprUseId>,
    pub(crate) max_height: Option<CheckedExprUseId>,
    pub(crate) clip: Option<CheckedExprUseId>,
    pub(crate) min_cell: Option<CheckedExprUseId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedScrollAnchor {
    Start,
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedScrollCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedScrollStatus {
    Active,
    Hovered,
    Dragged,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedScrollSelector {
    pub(crate) horizontal_interaction: Option<bool>,
    pub(crate) vertical_interaction: Option<bool>,
    pub(crate) horizontal_disabled: Option<bool>,
    pub(crate) vertical_disabled: Option<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedScrollStatusStyle {
    pub(crate) status: ResolvedScrollStatus,
    pub(crate) selector: ResolvedScrollSelector,
    pub(crate) container: ResolvedContainerSurface,
    pub(crate) horizontal_rail: ResolvedContainerSurface,
    pub(crate) horizontal_scroller: ResolvedContainerSurface,
    pub(crate) vertical_rail: ResolvedContainerSurface,
    pub(crate) vertical_scroller: ResolvedContainerSurface,
    pub(crate) gap: Option<ResolvedContainerBackground>,
    pub(crate) auto_scroll: ResolvedContainerSurface,
    pub(crate) auto_scroll_icon: Option<ResolvedThemeColor>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedScrollLayout {
    pub(crate) direction: ResolvedScrollDirection,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) hidden_bar: bool,
    pub(crate) bar_width: Option<CheckedExprUseId>,
    pub(crate) bar_margin: Option<CheckedExprUseId>,
    pub(crate) scroller_width: Option<CheckedExprUseId>,
    pub(crate) bar_spacing: Option<CheckedExprUseId>,
    pub(crate) anchor_x: ResolvedScrollAnchor,
    pub(crate) anchor_y: ResolvedScrollAnchor,
    pub(crate) auto_scroll: Option<CheckedExprUseId>,
    pub(crate) route: Option<ResolvedInteractionRoute>,
    pub(crate) viewport_route: Option<ResolvedInteractionRoute>,
    pub(crate) custom_style: Option<ResolvedScrollCustomStyle>,
    pub(crate) styles: Vec<ResolvedScrollStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedLayoutMode {
    Linear(ResolvedLinearLayout),
    Grid(ResolvedGridLayout),
    Stack(ResolvedStackLayout),
    Hover(ResolvedHoverLayout),
    Flex(ResolvedFlexLayout),
    Scroll(Box<ResolvedScrollLayout>),
}

/// The draw-time hover container: base + reveal, an optional hover tint, and
/// the application's own "held open" verdict.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedHoverLayout {
    pub(crate) tint: Option<ResolvedThemeColor>,
    pub(crate) radius: f64,
    pub(crate) open: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLayout {
    pub(crate) id: ViewId,
    pub(crate) mode: ResolvedLayoutMode,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

struct LayoutOperands<'a> {
    lowerer: &'a Lowerer,
    layout: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl LayoutOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("layout {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.layout,
            index: self.next,
        });
        self.next += 1;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("layout {label} expression ID is invalid"),
                )
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("layout {label} expression contract diverged"),
            ));
        }
        Ok((expression, retained.source.clone()))
    }

    fn take(&mut self, expected: &Type, label: &str) -> Result<CheckedExprUseId, Error> {
        self.take_where(label, |actual| actual == expected)
            .map(|(expression, _)| expression)
    }

    fn optional<T>(
        &mut self,
        value: Option<&T>,
        expected: &Type,
        label: &str,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value.map(|_| self.take(expected, label)).transpose()
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self.lowerer.invariant(
                self.span,
                "layout left checked option expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_layout(
        &mut self,
        kind: Layout,
        options: &LayoutOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Layout,
            crate::ast::layout_semantic_key(kind, options),
            span,
            outer_component,
        )?;
        let checked_layout = self
            .facts
            .layout(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "layout has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let expected_roots = crate::ast::layout_expression_roots(options).len();
        if checked.option_expressions.len() != expected_roots {
            return Err(self.invariant(span, "layout expression cardinality diverged"));
        }
        let mut values = LayoutOperands {
            lowerer: self,
            layout: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };

        let columns = values.optional(options.columns.as_ref(), &Type::I64, "columns")?;
        let clip = values.optional(options.clip.as_ref(), &Type::Bool, "clip")?;
        let hover_open = values.optional(options.hover_open.as_ref(), &Type::Bool, "hover open")?;
        let width = Self::resolve_layout_length(&mut values, &options.width, "width")?;
        let height = Self::resolve_layout_length(&mut values, &options.height, "height")?;
        let spacing = values.optional(options.spacing.as_ref(), &Type::F64, "spacing")?;
        let padding = Self::resolve_layout_padding(&mut values, &options.padding)?;
        let max_width = values.optional(options.max_width.as_ref(), &Type::F64, "max-width")?;
        let max_height = values.optional(options.max_height.as_ref(), &Type::F64, "max-height")?;
        let virtual_row =
            values.optional(options.virtual_row.as_ref(), &Type::F64, "virtual row")?;
        let wrap_spacing =
            values.optional(options.wrap_spacing.as_ref(), &Type::F64, "wrap spacing")?;
        let (row_gap, column_gap) = options.flexbox.as_ref().map_or_else(
            || Ok((None, None)),
            |flexbox| {
                Ok((
                    values.optional(flexbox.row_gap.as_ref(), &Type::F64, "row gap")?,
                    values.optional(flexbox.column_gap.as_ref(), &Type::F64, "column gap")?,
                ))
            },
        )?;
        let min_cell = values.optional(options.min_cell.as_ref(), &Type::F64, "minimum cell")?;
        let max_cell = values.optional(options.max_cell.as_ref(), &Type::F64, "maximum cell")?;
        let grid_height = options
            .grid_height
            .as_ref()
            .map(|height| Self::resolve_grid_height(&mut values, height))
            .transpose()?;
        let scroll = options
            .scroll
            .as_ref()
            .map(|scroll| {
                self.resolve_scroll_layout(&mut values, scroll, &checked_layout, origin, span)
            })
            .transpose()?;
        values.finish()?;

        let mut route_index = 0usize;
        let routes = crate::ast::layout_routes(options);
        let (route, viewport_route) = if let Some(source) = &options.scroll {
            (
                self.lower_optional_interaction_route(
                    &source.route,
                    &checked,
                    &routes,
                    &mut route_index,
                    id,
                    scope,
                )?,
                self.lower_optional_interaction_route(
                    &source.viewport_route,
                    &checked,
                    &routes,
                    &mut route_index,
                    id,
                    scope,
                )?,
            )
        } else {
            (None, None)
        };
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "layout left checked routes unconsumed"));
        }

        let alignment = |value| match value {
            FlexAlignment::Start => ResolvedContainerAlignment::Start,
            FlexAlignment::Center => ResolvedContainerAlignment::Center,
            FlexAlignment::End => ResolvedContainerAlignment::End,
        };
        let mode = if kind == Layout::Grid && min_cell.is_some() {
            ResolvedLayoutMode::Flex(ResolvedFlexLayout {
                direction: ResolvedFlexDirection::Row,
                wrap: ResolvedFlexWrap::Wrap,
                justify_content: None,
                align_items: None,
                align_content: None,
                spacing,
                wrap_spacing,
                row_gap: None,
                column_gap: None,
                padding,
                width: width.or(Some(ResolvedContainerLength::Fill)),
                height,
                max_width,
                max_height,
                clip,
                min_cell,
            })
        } else {
            match kind {
                Layout::Column | Layout::Row if options.flexbox.is_some() => {
                    let flexbox = options.flexbox.as_ref().unwrap();
                    ResolvedLayoutMode::Flex(ResolvedFlexLayout {
                        direction: Self::resolve_flex_direction(flexbox.direction),
                        wrap: Self::resolve_flex_wrap(flexbox.wrap),
                        justify_content: flexbox
                            .justify_content
                            .map(Self::resolve_flex_content_alignment),
                        align_items: flexbox.align_items.map(Self::resolve_flex_item_alignment),
                        align_content: flexbox
                            .align_content
                            .map(Self::resolve_flex_content_alignment),
                        spacing,
                        wrap_spacing,
                        row_gap,
                        column_gap,
                        padding,
                        width,
                        height,
                        max_width,
                        max_height,
                        clip,
                        min_cell: None,
                    })
                }
                Layout::Column | Layout::Row => ResolvedLayoutMode::Linear(ResolvedLinearLayout {
                    axis: if kind == Layout::Column {
                        ResolvedLinearAxis::Column
                    } else {
                        ResolvedLinearAxis::Row
                    },
                    spacing,
                    padding,
                    width,
                    height,
                    max_width,
                    virtual_row,
                    align: options.align.map(alignment),
                    clip,
                    wrap: options.wrap,
                    wrap_spacing,
                    wrap_align: options.wrap_align.map(alignment),
                }),
                Layout::Grid => ResolvedLayoutMode::Grid(ResolvedGridLayout {
                    columns,
                    spacing,
                    width: width.map(|width| match width {
                        ResolvedContainerLength::FixedF64(expression) => expression,
                        _ => unreachable!("checker keeps grid width numeric"),
                    }),
                    height: grid_height,
                    max_cell,
                }),
                Layout::Stack => ResolvedLayoutMode::Stack(ResolvedStackLayout {
                    under: options.under,
                    clip,
                    width,
                    height,
                }),
                Layout::Hover => ResolvedLayoutMode::Hover(ResolvedHoverLayout {
                    tint: options
                        .hover_tint
                        .as_deref()
                        .map(|tint| self.resolve_theme_color(tint, span))
                        .transpose()?,
                    radius: options.hover_radius.unwrap_or(0.0),
                    open: hover_open,
                }),
                Layout::Scroll => {
                    let mut scroll = scroll
                        .ok_or_else(|| self.invariant(span, "scroll layout lost its options"))?;
                    scroll.route = route;
                    scroll.viewport_route = viewport_route;
                    ResolvedLayoutMode::Scroll(Box::new(scroll))
                }
            }
        };
        let utility_style = self
            .styles
            .style_use(span)
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "layout utility style site is not normalized"))?;
        let resolved = ResolvedLayout {
            id,
            mode,
            utility_style,
            origin,
        };
        if self.layouts.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "layout was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_layout_padding(
        values: &mut LayoutOperands<'_>,
        padding: &PaddingOptions,
    ) -> Result<ResolvedContainerPadding, Error> {
        Ok(ResolvedContainerPadding {
            all: values.optional(padding.all.as_ref(), &Type::F64, "padding")?,
            x: values.optional(padding.x.as_ref(), &Type::F64, "padding-x")?,
            y: values.optional(padding.y.as_ref(), &Type::F64, "padding-y")?,
            top: values.optional(padding.top.as_ref(), &Type::F64, "padding-top")?,
            right: values.optional(padding.right.as_ref(), &Type::F64, "padding-right")?,
            bottom: values.optional(padding.bottom.as_ref(), &Type::F64, "padding-bottom")?,
            left: values.optional(padding.left.as_ref(), &Type::F64, "padding-left")?,
        })
    }

    fn resolve_layout_length(
        values: &mut LayoutOperands<'_>,
        length: &Option<LengthValue>,
        label: &str,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match length {
            None => None,
            Some(LengthValue::Fill) => Some(ResolvedContainerLength::Fill),
            Some(LengthValue::FillPortion(portion)) => {
                Some(ResolvedContainerLength::FillPortion(*portion))
            }
            Some(LengthValue::Shrink) => Some(ResolvedContainerLength::Shrink),
            Some(LengthValue::Fixed(_)) => {
                let (expression, source) = values
                    .take_where(label, |actual| matches!(actual, Type::F64 | Type::Length))?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("validated layout length type"),
                })
            }
        })
    }

    fn resolve_grid_height(
        values: &mut LayoutOperands<'_>,
        height: &GridSizing,
    ) -> Result<ResolvedGridHeight, Error> {
        Ok(match height {
            GridSizing::AspectRatio { .. } => ResolvedGridHeight::AspectRatio {
                width: values.take(&Type::F64, "grid aspect width")?,
                height: values.take(&Type::F64, "grid aspect height")?,
            },
            GridSizing::EvenlyDistribute(length) => ResolvedGridHeight::EvenlyDistribute(
                Self::resolve_layout_length(values, &Some(length.clone()), "grid height")?
                    .expect("grid height is present"),
            ),
        })
    }

    fn resolve_scroll_layout(
        &self,
        values: &mut LayoutOperands<'_>,
        scroll: &ScrollOptions,
        checked: &CheckedLayout,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedScrollLayout, Error> {
        let width = Self::resolve_layout_length(values, &scroll.width, "scroll width")?;
        let height = Self::resolve_layout_length(values, &scroll.height, "scroll height")?;
        let bar_width = values.optional(scroll.bar_width.as_ref(), &Type::F64, "bar width")?;
        let bar_margin = values.optional(scroll.bar_margin.as_ref(), &Type::F64, "bar margin")?;
        let scroller_width =
            values.optional(scroll.scroller_width.as_ref(), &Type::F64, "scroller width")?;
        let bar_spacing =
            values.optional(scroll.bar_spacing.as_ref(), &Type::F64, "bar spacing")?;
        let auto_scroll =
            values.optional(scroll.auto_scroll.as_ref(), &Type::Bool, "auto-scroll")?;
        let custom_style = scroll
            .custom_style
            .as_ref()
            .map(|style| {
                let function = checked.scroll_style.ok_or_else(|| {
                    self.invariant(span, "scroll custom style lost its checked extern ID")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "scroll style extern disappeared"))?;
                if declaration.name != style.function
                    || declaration.kind != ExternKind::ScrollStyle
                    || declaration.params.len() != style.args.len()
                {
                    return Err(self.invariant(span, "scroll style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "scroll style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedScrollCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if custom_style.is_none() != checked.scroll_style.is_none() {
            return Err(self.invariant(span, "scroll custom style presence diverged"));
        }
        if checked.style_origins.len() != scroll.styles.len() {
            return Err(self.invariant(span, "scroll style origin count diverged"));
        }
        let styles = scroll
            .styles
            .iter()
            .zip(&checked.style_origins)
            .map(|(style, style_origin)| {
                let retained_origin = self.origins.try_get(*style_origin).ok_or_else(|| {
                    self.invariant(&style.span, "scroll style origin is outside its arena")
                })?;
                if retained_origin.parent != Some(origin) {
                    return Err(self.invariant(&style.span, "scroll style origin parent diverged"));
                }
                self.resolve_scroll_status_style(values, style, *style_origin)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let direction = match scroll.direction {
            ScrollDirection::Vertical => ResolvedScrollDirection::Vertical,
            ScrollDirection::Horizontal => ResolvedScrollDirection::Horizontal,
            ScrollDirection::Both => ResolvedScrollDirection::Both,
        };
        let anchor = |anchor| match anchor {
            ScrollAnchor::Start => ResolvedScrollAnchor::Start,
            ScrollAnchor::End => ResolvedScrollAnchor::End,
        };
        Ok(ResolvedScrollLayout {
            direction,
            width,
            height,
            hidden_bar: scroll.hidden_bar,
            bar_width,
            bar_margin,
            scroller_width,
            bar_spacing,
            anchor_x: anchor(scroll.anchor_x),
            anchor_y: anchor(scroll.anchor_y),
            auto_scroll,
            route: None,
            viewport_route: None,
            custom_style,
            styles,
        })
    }

    fn resolve_scroll_status_style(
        &self,
        values: &mut LayoutOperands<'_>,
        style: &ScrollStatusStyle,
        _origin: OriginId,
    ) -> Result<ResolvedScrollStatusStyle, Error> {
        let status = match style.status {
            ScrollStatus::Active => ResolvedScrollStatus::Active,
            ScrollStatus::Hovered => ResolvedScrollStatus::Hovered,
            ScrollStatus::Dragged => ResolvedScrollStatus::Dragged,
        };
        let container = self.resolve_layout_surface(values, &style.container, &style.span)?;
        let horizontal_rail =
            self.resolve_layout_surface(values, &style.horizontal_rail.rail, &style.span)?;
        let horizontal_scroller =
            self.resolve_layout_surface(values, &style.horizontal_rail.scroller, &style.span)?;
        let vertical_rail =
            self.resolve_layout_surface(values, &style.vertical_rail.rail, &style.span)?;
        let vertical_scroller =
            self.resolve_layout_surface(values, &style.vertical_rail.scroller, &style.span)?;
        let auto_scroll = self.resolve_layout_surface(values, &style.auto_scroll, &style.span)?;
        let gap = style
            .gap
            .as_ref()
            .map(|background| self.resolve_layout_background(values, background, &style.span))
            .transpose()?;
        Ok(ResolvedScrollStatusStyle {
            status,
            selector: ResolvedScrollSelector {
                horizontal_interaction: style.horizontal_interaction,
                vertical_interaction: style.vertical_interaction,
                horizontal_disabled: style.horizontal_disabled,
                vertical_disabled: style.vertical_disabled,
            },
            container,
            horizontal_rail,
            horizontal_scroller,
            vertical_rail,
            vertical_scroller,
            gap,
            auto_scroll,
            auto_scroll_icon: style
                .auto_scroll_icon
                .as_deref()
                .map(|color| self.resolve_theme_color(color, &style.span))
                .transpose()?,
            #[cfg(test)]
            origin: _origin,
        })
    }

    fn resolve_layout_surface(
        &self,
        values: &mut LayoutOperands<'_>,
        style: &ContainerStyleOptions,
        span: &Span,
    ) -> Result<ResolvedContainerSurface, Error> {
        let background = style
            .background
            .as_ref()
            .map(|background| self.resolve_layout_background(values, background, span))
            .transpose()?;
        Ok(ResolvedContainerSurface {
            background,
            text_color: style
                .text_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_color: style
                .border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_width: values.optional(
                style.border_width.as_ref(),
                &Type::F64,
                "surface border width",
            )?,
            radius: ResolvedContainerRadius {
                all: values.optional(style.radius.as_ref(), &Type::F64, "surface radius")?,
                top_left: values.optional(
                    style.radius_top_left.as_ref(),
                    &Type::F64,
                    "surface top-left radius",
                )?,
                top_right: values.optional(
                    style.radius_top_right.as_ref(),
                    &Type::F64,
                    "surface top-right radius",
                )?,
                bottom_right: values.optional(
                    style.radius_bottom_right.as_ref(),
                    &Type::F64,
                    "surface bottom-right radius",
                )?,
                bottom_left: values.optional(
                    style.radius_bottom_left.as_ref(),
                    &Type::F64,
                    "surface bottom-left radius",
                )?,
            },
            shadow_color: style
                .shadow_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            shadow_x: values.optional(style.shadow_x.as_ref(), &Type::F64, "surface shadow x")?,
            shadow_y: values.optional(style.shadow_y.as_ref(), &Type::F64, "surface shadow y")?,
            shadow_blur: values.optional(
                style.shadow_blur.as_ref(),
                &Type::F64,
                "surface shadow blur",
            )?,
            pixel_snap: values.optional(
                style.pixel_snap.as_ref(),
                &Type::Bool,
                "surface pixel snap",
            )?,
        })
    }

    fn resolve_layout_background(
        &self,
        values: &mut LayoutOperands<'_>,
        background: &BackgroundValue,
        span: &Span,
    ) -> Result<ResolvedContainerBackground, Error> {
        Ok(match background {
            BackgroundValue::Color(color) => {
                ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { stops, .. } => {
                let angle = values.take(&Type::F64, "background angle")?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        Ok(ResolvedContainerGradientStop {
                            color: self.resolve_theme_color(&stop.color, span)?,
                            offset: values.take(&Type::F64, "background stop")?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedContainerBackground::Linear { angle, stops }
            }
        })
    }

    fn resolve_flex_direction(direction: FlexDirectionValue) -> ResolvedFlexDirection {
        match direction {
            FlexDirectionValue::Row => ResolvedFlexDirection::Row,
            FlexDirectionValue::RowReverse => ResolvedFlexDirection::RowReverse,
            FlexDirectionValue::Column => ResolvedFlexDirection::Column,
            FlexDirectionValue::ColumnReverse => ResolvedFlexDirection::ColumnReverse,
        }
    }

    fn resolve_flex_wrap(wrap: FlexWrapValue) -> ResolvedFlexWrap {
        match wrap {
            FlexWrapValue::NoWrap => ResolvedFlexWrap::NoWrap,
            FlexWrapValue::Wrap => ResolvedFlexWrap::Wrap,
            FlexWrapValue::WrapReverse => ResolvedFlexWrap::WrapReverse,
        }
    }

    fn resolve_flex_item_alignment(align: FlexItemAlignment) -> ResolvedContainerFlexAlignment {
        match align {
            FlexItemAlignment::Start => ResolvedContainerFlexAlignment::Start,
            FlexItemAlignment::End => ResolvedContainerFlexAlignment::End,
            FlexItemAlignment::FlexStart => ResolvedContainerFlexAlignment::FlexStart,
            FlexItemAlignment::FlexEnd => ResolvedContainerFlexAlignment::FlexEnd,
            FlexItemAlignment::Center => ResolvedContainerFlexAlignment::Center,
            FlexItemAlignment::Baseline => ResolvedContainerFlexAlignment::Baseline,
            FlexItemAlignment::Stretch => ResolvedContainerFlexAlignment::Stretch,
        }
    }

    fn resolve_flex_content_alignment(align: FlexContentAlignment) -> ResolvedFlexContentAlignment {
        match align {
            FlexContentAlignment::Start => ResolvedFlexContentAlignment::Start,
            FlexContentAlignment::End => ResolvedFlexContentAlignment::End,
            FlexContentAlignment::FlexStart => ResolvedFlexContentAlignment::FlexStart,
            FlexContentAlignment::FlexEnd => ResolvedFlexContentAlignment::FlexEnd,
            FlexContentAlignment::Center => ResolvedFlexContentAlignment::Center,
            FlexContentAlignment::Stretch => ResolvedFlexContentAlignment::Stretch,
            FlexContentAlignment::SpaceBetween => ResolvedFlexContentAlignment::SpaceBetween,
            FlexContentAlignment::SpaceAround => ResolvedFlexContentAlignment::SpaceAround,
            FlexContentAlignment::SpaceEvenly => ResolvedFlexContentAlignment::SpaceEvenly,
        }
    }
}
