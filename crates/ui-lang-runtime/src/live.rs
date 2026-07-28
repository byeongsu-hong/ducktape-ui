//! Development-only live-plan loading and rendering.

use iced::advanced::text;
use iced::widget;
use iced::{Element, Subscription};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use ui_lang_live_protocol::{
    LiveBinaryOp, LiveExpression, LivePlan, LiveProgramContract, LiveReloadDecision,
    LiveRestartReason, LiveRoute, LiveStateChange, LiveUnaryOp, LiveView, evaluate_live_reload,
};
pub use ui_lang_live_protocol::{LiveEvent, LiveValue};

pub const PLAN_PATH_ENV: &str = "ICE_LIVE_PLAN";
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LivePoll {
    Disabled,
    Unchanged,
    Reloaded {
        revision: u64,
        state_changes: Vec<LiveStateChange>,
    },
    RestartRequired {
        revision: u64,
        reasons: Vec<LiveRestartReason>,
    },
    Error(String),
}

/// Keeps the last-known-good live plan for one generated application.
#[derive(Debug)]
pub struct LiveRuntime {
    contract: Option<LiveProgramContract>,
    path: Option<PathBuf>,
    last_payload: Option<String>,
    plan: Option<LivePlan>,
}

impl LiveRuntime {
    /// Creates a runtime from a compiler-emitted contract and `ICE_LIVE_PLAN`.
    pub fn new(contract_json: &str) -> Self {
        let Some(path) = std::env::var_os(PLAN_PATH_ENV).map(PathBuf::from) else {
            return Self {
                contract: None,
                path: None,
                last_payload: None,
                plan: None,
            };
        };
        let contract = serde_json::from_str(contract_json)
            .expect("the Ice compiler emits a valid live contract");
        let mut runtime = Self::with_path(contract, Some(path));
        runtime.poll_and_report();
        runtime
    }

    pub fn with_path(contract: LiveProgramContract, path: Option<PathBuf>) -> Self {
        Self {
            contract: Some(contract),
            path,
            last_payload: None,
            plan: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.path.is_some()
    }

    pub fn active_revision(&self) -> Option<u64> {
        self.plan.as_ref().map(|plan| plan.revision)
    }

    pub fn subscription(&self) -> Subscription<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.is_enabled() {
            return Subscription::run(tick_stream);
        }
        Subscription::none()
    }

    /// Reads and atomically accepts a newer compatible plan.
    ///
    /// Parse errors and restart-only revisions leave the active plan untouched.
    pub fn poll(&mut self) -> LivePoll {
        let Some(path) = &self.path else {
            return LivePoll::Disabled;
        };
        let payload = match fs::read_to_string(path) {
            Ok(payload) => payload,
            Err(error) => {
                return LivePoll::Error(format!(
                    "cannot read live plan {}: {error}",
                    path.display()
                ));
            }
        };
        if self.last_payload.as_deref() == Some(payload.as_str()) {
            return LivePoll::Unchanged;
        }
        self.last_payload = Some(payload.clone());
        let plan = match serde_json::from_str::<LivePlan>(&payload) {
            Ok(plan) => plan,
            Err(error) => return LivePoll::Error(format!("invalid live plan: {error}")),
        };
        if self
            .plan
            .as_ref()
            .is_some_and(|current| plan.revision <= current.revision)
        {
            return LivePoll::Unchanged;
        }

        let contract = self
            .contract
            .as_ref()
            .expect("an enabled live runtime has a compiler contract");
        match evaluate_live_reload(contract, &plan.contract) {
            LiveReloadDecision::Reload { state_changes } => {
                let revision = plan.revision;
                self.contract = Some(plan.contract.clone());
                self.plan = Some(plan);
                LivePoll::Reloaded {
                    revision,
                    state_changes,
                }
            }
            LiveReloadDecision::RestartRequired { reasons } => LivePoll::RestartRequired {
                revision: plan.revision,
                reasons,
            },
        }
    }

    /// Polls once, reports development diagnostics, and returns whether a new
    /// plan was installed.
    pub fn poll_and_report(&mut self) -> bool {
        match self.poll() {
            LivePoll::Reloaded { revision, .. } => {
                eprintln!("ice live: installed revision {revision}");
                true
            }
            LivePoll::RestartRequired { revision, reasons } => {
                let reasons = reasons
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                eprintln!("ice live: revision {revision} requires restart: {reasons}");
                false
            }
            LivePoll::Error(error) => {
                eprintln!("ice live: {error}");
                false
            }
            LivePoll::Disabled | LivePoll::Unchanged => false,
        }
    }

    pub fn render<Message, Theme, Renderer, Event>(
        &self,
        values: &BTreeMap<String, LiveValue>,
        event: Event,
    ) -> Option<Element<'static, Message, Theme, Renderer>>
    where
        Message: Clone + 'static,
        Theme: widget::text::Catalog + widget::button::Catalog + 'static,
        Renderer: text::Renderer + 'static,
        Event: Fn(LiveEvent) -> Message + Clone + 'static,
    {
        self.plan
            .as_ref()
            .and_then(|plan| plan.view.as_ref())
            .and_then(|view| {
                render_view::<Message, Theme, Renderer, Event>(view, values, &event).ok()
            })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn tick_stream() -> iced::futures::channel::mpsc::UnboundedReceiver<()> {
    let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(POLL_INTERVAL);
            if sender.unbounded_send(()).is_err() {
                break;
            }
        }
    });
    receiver
}

fn render_view<Message, Theme, Renderer, Event>(
    view: &LiveView,
    values: &BTreeMap<String, LiveValue>,
    event: &Event,
) -> Result<Element<'static, Message, Theme, Renderer>, String>
where
    Message: Clone + 'static,
    Theme: widget::text::Catalog + widget::button::Catalog + 'static,
    Renderer: text::Renderer + 'static,
    Event: Fn(LiveEvent) -> Message + Clone + 'static,
{
    match view {
        LiveView::Column { key, children } => {
            let layout = widget::column(render_children::<Message, Theme, Renderer, Event>(
                children, values, event,
            )?);
            Ok(crate::accessible(
                layout,
                crate::StableId::new(key),
                crate::Role::GenericContainer,
            )
            .logical_id(key.clone())
            .into())
        }
        LiveView::Row { key, children } => {
            let layout = widget::row(render_children::<Message, Theme, Renderer, Event>(
                children, values, event,
            )?);
            Ok(crate::accessible(
                layout,
                crate::StableId::new(key),
                crate::Role::GenericContainer,
            )
            .logical_id(key.clone())
            .into())
        }
        LiveView::Text { key, value } => {
            let value = display_value(&evaluate(value, values)?);
            let text = widget::text::<Theme, Renderer>(value.clone());
            Ok(
                crate::accessible(text, crate::StableId::new(key), crate::Role::Label)
                    .logical_id(key.clone())
                    .value(value)
                    .into(),
            )
        }
        LiveView::Button {
            key,
            label,
            disabled,
            route,
        } => {
            let disabled = disabled
                .as_ref()
                .map(|expression| evaluate_bool(expression, values))
                .transpose()?
                .unwrap_or(false);
            let activate = event(evaluate_route(route, values)?);
            let button = widget::button(widget::text::<Theme, Renderer>(label.clone()))
                .on_press_maybe((!disabled).then(|| activate.clone()));
            Ok(
                crate::accessible(button, crate::StableId::new(key), crate::Role::Button)
                    .logical_id(key.clone())
                    .focus_id(widget::Id::from(key.clone()))
                    .label(label.clone())
                    .disabled(disabled)
                    .on_activate_maybe((!disabled).then_some(activate))
                    .into(),
            )
        }
        LiveView::If { .. } => Err("live if must be a direct layout child".into()),
    }
}

fn render_children<Message, Theme, Renderer, Event>(
    children: &[LiveView],
    values: &BTreeMap<String, LiveValue>,
    event: &Event,
) -> Result<Vec<Element<'static, Message, Theme, Renderer>>, String>
where
    Message: Clone + 'static,
    Theme: widget::text::Catalog + widget::button::Catalog + 'static,
    Renderer: text::Renderer + 'static,
    Event: Fn(LiveEvent) -> Message + Clone + 'static,
{
    let mut rendered = Vec::new();
    for child in children {
        if let LiveView::If {
            condition,
            children,
        } = child
        {
            if evaluate_bool(condition, values)? {
                rendered.extend(render_children::<Message, Theme, Renderer, Event>(
                    children, values, event,
                )?);
            }
        } else {
            rendered.push(render_view::<Message, Theme, Renderer, Event>(
                child, values, event,
            )?);
        }
    }
    Ok(rendered)
}

fn evaluate_route(
    route: &LiveRoute,
    values: &BTreeMap<String, LiveValue>,
) -> Result<LiveEvent, String> {
    Ok(LiveEvent {
        handler: route.handler.clone(),
        args: route
            .args
            .iter()
            .map(|argument| evaluate(argument, values))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn evaluate_bool(
    expression: &LiveExpression,
    values: &BTreeMap<String, LiveValue>,
) -> Result<bool, String> {
    match evaluate(expression, values)? {
        LiveValue::Bool(value) => Ok(value),
        value => Err(format!("expected bool, found {}", value_kind(&value))),
    }
}

fn evaluate(
    expression: &LiveExpression,
    values: &BTreeMap<String, LiveValue>,
) -> Result<LiveValue, String> {
    match expression {
        LiveExpression::Bool(value) => Ok(LiveValue::Bool(*value)),
        LiveExpression::I64(value) => Ok(LiveValue::I64(*value)),
        LiveExpression::F64(value) => Ok(LiveValue::F64(*value)),
        LiveExpression::String(value) => Ok(LiveValue::String(value.clone())),
        LiveExpression::Path(name) => values
            .get(name)
            .cloned()
            .ok_or_else(|| format!("live value `{name}` is unavailable")),
        LiveExpression::Unary { op, value } => match (op, evaluate(value, values)?) {
            (LiveUnaryOp::Not, LiveValue::Bool(value)) => Ok(LiveValue::Bool(!value)),
            (LiveUnaryOp::Neg, LiveValue::I64(value)) => value
                .checked_neg()
                .map(LiveValue::I64)
                .ok_or_else(|| "integer negation overflowed".into()),
            (LiveUnaryOp::Neg, LiveValue::F64(value)) => Ok(LiveValue::F64(-value)),
            (op, value) => Err(format!(
                "operator {op:?} does not accept {}",
                value_kind(&value)
            )),
        },
        LiveExpression::Binary { left, op, right } => {
            let left = evaluate(left, values)?;
            match op {
                LiveBinaryOp::And => match left {
                    LiveValue::Bool(false) => Ok(LiveValue::Bool(false)),
                    LiveValue::Bool(true) => evaluate_bool(right, values).map(LiveValue::Bool),
                    value => Err(format!(
                        "operator And does not accept {}",
                        value_kind(&value)
                    )),
                },
                LiveBinaryOp::Or => match left {
                    LiveValue::Bool(true) => Ok(LiveValue::Bool(true)),
                    LiveValue::Bool(false) => evaluate_bool(right, values).map(LiveValue::Bool),
                    value => Err(format!(
                        "operator Or does not accept {}",
                        value_kind(&value)
                    )),
                },
                _ => evaluate_binary(*op, left, evaluate(right, values)?),
            }
        }
    }
}

fn evaluate_binary(
    op: LiveBinaryOp,
    left: LiveValue,
    right: LiveValue,
) -> Result<LiveValue, String> {
    use LiveBinaryOp as Op;
    use LiveValue as Value;
    let left_kind = value_kind(&left);
    let right_kind = value_kind(&right);
    let invalid = || {
        Err(format!(
            "operator {op:?} does not accept {left_kind} and {right_kind}"
        ))
    };
    match (op, &left, &right) {
        (Op::Eq, _, _) => return Ok(Value::Bool(left == right)),
        (Op::NotEq, _, _) => return Ok(Value::Bool(left != right)),
        (Op::Add, Value::String(left), Value::String(right)) => {
            return Ok(Value::String(format!("{left}{right}")));
        }
        (Op::Lt, Value::String(left), Value::String(right)) => {
            return Ok(Value::Bool(left < right));
        }
        (Op::LtEq, Value::String(left), Value::String(right)) => {
            return Ok(Value::Bool(left <= right));
        }
        (Op::Gt, Value::String(left), Value::String(right)) => {
            return Ok(Value::Bool(left > right));
        }
        (Op::GtEq, Value::String(left), Value::String(right)) => {
            return Ok(Value::Bool(left >= right));
        }
        _ => {}
    }
    match (left, right) {
        (Value::I64(left), Value::I64(right)) => match op {
            Op::Add => left.checked_add(right).map(Value::I64),
            Op::Sub => left.checked_sub(right).map(Value::I64),
            Op::Mul => left.checked_mul(right).map(Value::I64),
            Op::Div => left.checked_div(right).map(Value::I64),
            Op::Rem => left.checked_rem(right).map(Value::I64),
            Op::Lt => Some(Value::Bool(left < right)),
            Op::LtEq => Some(Value::Bool(left <= right)),
            Op::Gt => Some(Value::Bool(left > right)),
            Op::GtEq => Some(Value::Bool(left >= right)),
            _ => return invalid(),
        }
        .ok_or_else(|| format!("integer {op:?} failed")),
        (Value::F64(left), Value::F64(right)) => Ok(match op {
            Op::Add => Value::F64(left + right),
            Op::Sub => Value::F64(left - right),
            Op::Mul => Value::F64(left * right),
            Op::Div => Value::F64(left / right),
            Op::Rem => Value::F64(left % right),
            Op::Lt => Value::Bool(left < right),
            Op::LtEq => Value::Bool(left <= right),
            Op::Gt => Value::Bool(left > right),
            Op::GtEq => Value::Bool(left >= right),
            _ => return invalid(),
        }),
        _ => invalid(),
    }
}

fn value_kind(value: &LiveValue) -> &'static str {
    match value {
        LiveValue::Bool(_) => "bool",
        LiveValue::I64(_) => "i64",
        LiveValue::F64(_) => "f64",
        LiveValue::String(_) => "string",
    }
}

fn display_value(value: &LiveValue) -> String {
    match value {
        LiveValue::Bool(value) => value.to_string(),
        LiveValue::I64(value) => value.to_string(),
        LiveValue::F64(value) => value.to_string(),
        LiveValue::String(value) => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use ui_lang_live_protocol::{
        LIVE_PROTOCOL_VERSION, LiveProgramAbi, LiveProgramMode, LiveStateSchema,
    };

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    struct PlanFile(PathBuf);

    impl PlanFile {
        fn new() -> Self {
            let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
            Self(
                std::env::temp_dir()
                    .join(format!("ice-live-runtime-{}-{id}.json", std::process::id())),
            )
        }

        fn write(&self, payload: &str) {
            fs::write(&self.0, payload).unwrap();
        }
    }

    impl Drop for PlanFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn contract(app: &str) -> LiveProgramContract {
        LiveProgramContract {
            protocol_version: LIVE_PROTOCOL_VERSION,
            abi: LiveProgramAbi {
                app: app.into(),
                mode: LiveProgramMode::Application,
                bootstrap: String::new(),
                aot_semantics_digest: String::new(),
                named_types: Vec::new(),
                palette_types: Vec::new(),
                extern_structs: Vec::new(),
                extern_functions: Vec::new(),
            },
            state: LiveStateSchema::default(),
        }
    }

    fn plan(revision: u64, contract: LiveProgramContract, text: &str) -> LivePlan {
        LivePlan {
            revision,
            contract,
            view: Some(LiveView::Text {
                key: "Demo/live/root/text".into(),
                value: LiveExpression::String(text.into()),
            }),
        }
    }

    #[test]
    fn keeps_the_last_good_plan_across_invalid_and_restart_only_updates() {
        let file = PlanFile::new();
        let expected = contract("Demo");
        let mut runtime = LiveRuntime::with_path(expected.clone(), Some(file.0.clone()));
        file.write(&serde_json::to_string(&plan(1, expected.clone(), "one")).unwrap());

        assert!(matches!(
            runtime.poll(),
            LivePoll::Reloaded { revision: 1, .. }
        ));
        assert_eq!(runtime.active_revision(), Some(1));

        file.write("not json");
        assert!(matches!(runtime.poll(), LivePoll::Error(_)));
        assert_eq!(runtime.active_revision(), Some(1));

        file.write(&serde_json::to_string(&plan(2, contract("Other"), "two")).unwrap());
        assert!(matches!(
            runtime.poll(),
            LivePoll::RestartRequired {
                revision: 2,
                reasons
            } if reasons == vec![LiveRestartReason::ProgramIdentity]
        ));
        assert_eq!(runtime.active_revision(), Some(1));
    }

    #[test]
    fn ignores_duplicate_and_stale_revisions() {
        let file = PlanFile::new();
        let expected = contract("Demo");
        let mut runtime = LiveRuntime::with_path(expected.clone(), Some(file.0.clone()));
        file.write(&serde_json::to_string(&plan(2, expected.clone(), "two")).unwrap());
        assert!(matches!(
            runtime.poll(),
            LivePoll::Reloaded { revision: 2, .. }
        ));

        assert_eq!(runtime.poll(), LivePoll::Unchanged);
        file.write(&serde_json::to_string(&plan(1, expected, "one")).unwrap());
        assert_eq!(runtime.poll(), LivePoll::Unchanged);
        assert_eq!(runtime.active_revision(), Some(2));
    }

    #[test]
    fn evaluates_live_state_expressions_and_route_arguments() {
        let values = BTreeMap::from([
            ("count".into(), LiveValue::I64(4)),
            ("enabled".into(), LiveValue::Bool(true)),
        ]);
        let expression = LiveExpression::Binary {
            left: Box::new(LiveExpression::Path("count".into())),
            op: LiveBinaryOp::Add,
            right: Box::new(LiveExpression::I64(6)),
        };

        assert_eq!(evaluate(&expression, &values), Ok(LiveValue::I64(10)));
        assert_eq!(
            evaluate_route(
                &LiveRoute {
                    handler: "set".into(),
                    args: vec![expression, LiveExpression::Path("enabled".into())],
                },
                &values,
            ),
            Ok(LiveEvent {
                handler: "set".into(),
                args: vec![LiveValue::I64(10), LiveValue::Bool(true)],
            })
        );
    }

    #[test]
    fn short_circuits_boolean_live_expressions() {
        let expression = LiveExpression::Binary {
            left: Box::new(LiveExpression::Bool(false)),
            op: LiveBinaryOp::And,
            right: Box::new(LiveExpression::Path("missing".into())),
        };

        assert_eq!(
            evaluate(&expression, &BTreeMap::new()),
            Ok(LiveValue::Bool(false))
        );
    }
}
