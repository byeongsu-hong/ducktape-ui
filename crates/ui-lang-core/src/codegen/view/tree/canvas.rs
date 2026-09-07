use super::*;

pub(super) fn render(
    node: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let canvas = program.resolved_canvas(node)?;
    let options = &canvas.options;
    refuse_when(
        program,
        canvas.origin,
        !canvas.states.is_empty() || !canvas.events.is_empty(),
        "canvas-local state or native events",
    )?;
    refuse_when(
        program,
        canvas.origin,
        options.cache.is_some()
            || options.cache_group.is_some()
            || options.capture.is_some()
            || options.press.is_some()
            || options.release.is_some()
            || options.right_press.is_some()
            || options.right_release.is_some()
            || options.middle_press.is_some()
            || options.middle_release.is_some()
            || options.enter.is_some()
            || options.move_route.is_some()
            || options.scroll.is_some()
            || options.exit.is_some()
            || options.interaction.is_some()
            || options.interaction_expr.is_some()
            || options.interaction_outside.is_some(),
        "native canvas options; wrap declarative geometry in a mouse area for guest interaction",
    )?;
    let emitter = Emitter { program, canvas };
    let key = key_code(identity, "canvas", canvas.origin, scope, env, program)?;
    let width = emitter.length(options.width.as_ref(), env)?;
    let height = emitter.length(options.height.as_ref(), env)?;
    let commands = emitter.commands(&canvas.commands, env)?;
    Ok(format!(
        "{{ let mut __ice_geometry: ::std::vec::Vec<{WIRE}::CanvasCommand> = ::std::vec::Vec::new(); {commands} {WIRE}::Node::Canvas {{ key: {key}, width: {width}, height: {height}, commands: __ice_geometry }} }}"
    ))
}
struct Emitter<'a> {
    program: &'a LoweredProgram,
    canvas: &'a ResolvedCanvas,
}
impl Emitter<'_> {
    fn value(
        &self,
        id: CheckedExprUseId,
        env: &dyn BindingEnvironment,
        mode: ValueMode,
    ) -> Result<String, Error> {
        let expressions = self.program.expressions();
        let mut pending = vec![expressions.expression_use(id).root];
        while let Some(id) = pending.pop() {
            let forbidden =
                |local| local == self.canvas.width_local || local == self.canvas.height_local;
            match &expressions.expression(id).kind {
                ResolvedExpressionKind::Path {
                    root: ResolvedPathRoot::Local(local),
                    ..
                } if forbidden(*local) => {
                    return Err(refused(
                        self.program,
                        self.canvas.origin,
                        "canvas host-size bindings",
                    ));
                }
                ResolvedExpressionKind::List(values) => pending.extend(values),
                ResolvedExpressionKind::Call { arguments, .. } => {
                    for arg in arguments {
                        match arg {
                            ResolvedCallArgument::Value(value) => pending.push(*value),
                            ResolvedCallArgument::Binding(local) if forbidden(*local) => {
                                return Err(refused(
                                    self.program,
                                    self.canvas.origin,
                                    "canvas host-size bindings",
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                ResolvedExpressionKind::Unary { value, .. } => pending.push(*value),
                ResolvedExpressionKind::Binary { left, right, .. } => {
                    pending.extend([*left, *right])
                }
                _ => {}
            }
        }
        resolved_expr_use_code(self.program, id, env, mode)
    }
    fn number(&self, id: CheckedExprUseId, env: &dyn BindingEnvironment) -> Result<String, Error> {
        Ok(format!(
            "({}) as f32",
            self.value(id, env, ValueMode::Owned)?
        ))
    }
    fn pair(
        &self,
        a: CheckedExprUseId,
        b: CheckedExprUseId,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        Ok(format!(
            "[{},{}]",
            self.number(a, env)?,
            self.number(b, env)?
        ))
    }
    fn optional_number(
        &self,
        value: Option<CheckedExprUseId>,
        default: &str,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        value
            .map(|value| self.number(value, env))
            .transpose()
            .map(|value| value.unwrap_or_else(|| default.into()))
    }
    fn length(
        &self,
        length: Option<&ResolvedCanvasLength>,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        Ok(option_code(
            length
                .map(|length| {
                    Ok(match length {
                        ResolvedCanvasLength::Fill => format!("{WIRE}::Length::Fill"),
                        ResolvedCanvasLength::FillPortion(n) => {
                            format!("{WIRE}::Length::FillPortion({n})")
                        }
                        ResolvedCanvasLength::Shrink => format!("{WIRE}::Length::Shrink"),
                        ResolvedCanvasLength::Fixed {
                            source: Type::Length,
                            ..
                        } => {
                            return Err(refused(
                                self.program,
                                self.canvas.origin,
                                "a canvas `length` value",
                            ));
                        }
                        ResolvedCanvasLength::Fixed { expression, .. } => {
                            format!("{WIRE}::Length::Fixed({})", self.number(*expression, env)?)
                        }
                    })
                })
                .transpose()?,
        ))
    }
    fn radius(
        &self,
        radius: &ResolvedCanvasRadius,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        let all = self.optional_number(radius.all, "0.0", env)?;
        let values = [
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        ]
        .into_iter()
        .map(|v| self.optional_number(v, &all, env))
        .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("[{}]", values.join(",")))
    }
    fn color(&self, background: &ResolvedCanvasBackground) -> Result<String, Error> {
        match background {
            ResolvedCanvasBackground::Color(color) => Ok(rgba_code(color)),
            _ => Err(refused(
                self.program,
                self.canvas.origin,
                "canvas gradient paint",
            )),
        }
    }
    fn stroke(
        &self,
        stroke: &ResolvedCanvasStroke,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        let color = self.color(&stroke.style)?;
        let width = self.number(stroke.width, env)?;
        let cap = match stroke.cap {
            CanvasLineCap::Butt => "Butt",
            CanvasLineCap::Square => "Square",
            CanvasLineCap::Round => "Round",
        };
        let join = match stroke.join {
            CanvasLineJoin::Miter => "Miter",
            CanvasLineJoin::Round => "Round",
            CanvasLineJoin::Bevel => "Bevel",
        };
        let dash = stroke
            .dash
            .iter()
            .map(|v| self.number(*v, env))
            .collect::<Result<Vec<_>, _>>()?
            .join(",");
        let offset = self.value(stroke.dash_offset, env, ValueMode::Owned)?;
        Ok(format!(
            "{WIRE}::CanvasStroke {{ color: {color},width: {width},cap: {WIRE}::CanvasLineCap::{cap},join: {WIRE}::CanvasLineJoin::{join},dash:vec![{dash}],dash_offset:u32::try_from({offset}).unwrap_or(0) }}"
        ))
    }
    fn draw(
        &self,
        shape: String,
        paint: &ResolvedCanvasPaint,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        let fill = option_code(paint.fill.as_ref().map(|v| self.color(v)).transpose()?);
        let stroke = option_code(
            paint
                .stroke
                .as_ref()
                .map(|v| self.stroke(v, env))
                .transpose()?,
        );
        let even_odd = matches!(paint.fill_rule, CanvasFillRule::EvenOdd);
        Ok(format!(
            "__ice_geometry.push({WIRE}::CanvasCommand::Draw {{ shape: {WIRE}::CanvasShape::{shape},fill: {fill},even_odd: {even_odd},stroke: {stroke} }});"
        ))
    }
    fn commands(
        &self,
        commands: &[ResolvedCanvasCommand],
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        let mut code = String::new();
        for command in commands {
            match command {
                ResolvedCanvasCommand::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    paint,
                } => code.push_str(&self.draw(
                    format!(
                        "Rectangle {{ position: {},size: {},radius: {} }}",
                        self.pair(*x, *y, env)?,
                        self.pair(*width, *height, env)?,
                        self.radius(radius, env)?
                    ),
                    paint,
                    env,
                )?),
                ResolvedCanvasCommand::Circle {
                    x,
                    y,
                    radius,
                    paint,
                    ..
                } => code.push_str(&self.draw(
                    format!(
                        "Circle {{ center: {},radius: {} }}",
                        self.pair(*x, *y, env)?,
                        self.number(*radius, env)?
                    ),
                    paint,
                    env,
                )?),
                ResolvedCanvasCommand::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    stroke,
                } => {
                    let shape = format!(
                        "Line {{ from: {},to: {} }}",
                        self.pair(*x1, *y1, env)?,
                        self.pair(*x2, *y2, env)?
                    );
                    code.push_str(&self.draw(
                        shape,
                        &ResolvedCanvasPaint {
                            fill: None,
                            fill_rule: CanvasFillRule::NonZero,
                            stroke: Some(stroke.clone()),
                        },
                        env,
                    )?);
                }
                ResolvedCanvasCommand::Path { segments, paint } => {
                    let segments = segments
                        .iter()
                        .map(|s| self.segment(s, env))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(",");
                    code.push_str(&self.draw(format!("Path(vec![{segments}])"), paint, env)?);
                }
                ResolvedCanvasCommand::If {
                    condition,
                    commands,
                } => write!(
                    code,
                    "if {} {{ {} }}",
                    self.value(*condition, env, ValueMode::Owned)?,
                    self.commands(commands, env)?
                )
                .unwrap(),
                ResolvedCanvasCommand::For {
                    item,
                    items,
                    commands,
                    ..
                } => {
                    let items = self.value(*items, env, ValueMode::Borrowed)?;
                    let mut child = ScopedBindingEnv::new(env);
                    child.insert(
                        item.name.clone(),
                        Binding {
                            code: if copy_expression_type(&item.ty) {
                                format!("(*{})", item.name)
                            } else {
                                item.name.clone()
                            },
                            ty: item.ty.clone(),
                            local: false,
                            state: None,
                            owner: Some(BindingOwner::Local(item.local)),
                        },
                    );
                    write!(
                        code,
                        "for {} in {items}.iter() {{ {} }}",
                        item.name,
                        self.commands(commands, &child)?
                    )
                    .unwrap();
                }
                ResolvedCanvasCommand::Group {
                    transform,
                    commands,
                } => {
                    let x = self.optional_number(transform.x, "0.0", env)?;
                    let y = self.optional_number(transform.y, "0.0", env)?;
                    let rotate = self.optional_number(transform.rotate, "0.0", env)?;
                    let uniform = self.optional_number(transform.scale, "1.0_f32", env)?;
                    let sx = self.optional_number(transform.scale_x, "1.0_f32", env)?;
                    let sy = self.optional_number(transform.scale_y, "1.0_f32", env)?;
                    let clip = option_code(
                        transform
                            .clip
                            .as_ref()
                            .map(|values| {
                                values
                                    .iter()
                                    .map(|v| self.number(*v, env))
                                    .collect::<Result<Vec<_>, _>>()
                                    .map(|v| format!("[{}]", v.join(",")))
                            })
                            .transpose()?,
                    );
                    write!(code,"__ice_geometry.push({WIRE}::CanvasCommand::Push {{ translate:[{x},{y}],rotate: {rotate},scale:[({uniform}).max(f32::EPSILON)*({sx}).max(f32::EPSILON),({uniform}).max(f32::EPSILON)*({sy}).max(f32::EPSILON)],clip: {clip} }}); {} __ice_geometry.push({WIRE}::CanvasCommand::Pop);",self.commands(commands,env)?).unwrap();
                }
                ResolvedCanvasCommand::Text { .. } => {
                    return Err(refused(self.program, self.canvas.origin, "canvas text"));
                }
                ResolvedCanvasCommand::Image { .. } | ResolvedCanvasCommand::Svg { .. } => {
                    return Err(refused(
                        self.program,
                        self.canvas.origin,
                        "canvas image assets",
                    ));
                }
            }
        }
        Ok(code)
    }
    fn segment(
        &self,
        segment: &ResolvedCanvasPathSegment,
        env: &dyn BindingEnvironment,
    ) -> Result<String, Error> {
        let value = match segment {
            ResolvedCanvasPathSegment::Move(x, y) => format!("Move({})", self.pair(*x, *y, env)?),
            ResolvedCanvasPathSegment::Line(x, y) => format!("Line({})", self.pair(*x, *y, env)?),
            ResolvedCanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => format!(
                "Arc {{ center: {},radius: {},start: {},end: {} }}",
                self.pair(*x, *y, env)?,
                self.number(*radius, env)?,
                self.number(*start, env)?,
                self.number(*end, env)?
            ),
            ResolvedCanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => format!(
                "ArcTo {{ a: {},b: {},radius: {} }}",
                self.pair(*ax, *ay, env)?,
                self.pair(*bx, *by, env)?,
                self.number(*radius, env)?
            ),
            ResolvedCanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => format!(
                "Ellipse {{ center: {},radius: {},rotation: {},start: {},end: {} }}",
                self.pair(*x, *y, env)?,
                self.pair(*radius_x, *radius_y, env)?,
                self.number(*rotation, env)?,
                self.number(*start, env)?,
                self.number(*end, env)?
            ),
            ResolvedCanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => format!(
                "Bezier {{ a: {},b: {},end: {} }}",
                self.pair(*control_ax, *control_ay, env)?,
                self.pair(*control_bx, *control_by, env)?,
                self.pair(*x, *y, env)?
            ),
            ResolvedCanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => format!(
                "Quadratic {{ control: {},end: {} }}",
                self.pair(*control_x, *control_y, env)?,
                self.pair(*x, *y, env)?
            ),
            ResolvedCanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => format!(
                "Rectangle {{ position: {},size: {},radius:[0.0;4] }}",
                self.pair(*x, *y, env)?,
                self.pair(*width, *height, env)?
            ),
            ResolvedCanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => format!(
                "Rectangle {{ position: {},size: {},radius: {} }}",
                self.pair(*x, *y, env)?,
                self.pair(*width, *height, env)?,
                self.radius(radius, env)?
            ),
            ResolvedCanvasPathSegment::Circle { x, y, radius } => format!(
                "Circle {{ center: {},radius: {} }}",
                self.pair(*x, *y, env)?,
                self.number(*radius, env)?
            ),
            ResolvedCanvasPathSegment::Close => "Close".into(),
        };
        Ok(format!("{WIRE}::CanvasSegment::{value}"))
    }
}
