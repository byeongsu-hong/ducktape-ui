//! Versioned data shared by the Ice compiler, development server, and live runtime.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Version of the compiler/runtime contract used by Ice live development.
pub const LIVE_PROTOCOL_VERSION: u32 = 1;

/// The compile-time Rust boundary and the reloadable state schema of one app.
///
/// Whole-contract fingerprints are cache/diagnostic values. Reload safety
/// compares every structural ABI field; only opaque AOT behavior, which the
/// live runtime cannot inspect structurally, uses its explicit SHA-256 digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveProgramContract {
    pub protocol_version: u32,
    pub abi: LiveProgramAbi,
    pub state: LiveStateSchema,
}

impl LiveProgramContract {
    pub fn abi_fingerprint(&self) -> String {
        stable_fingerprint(&self.abi)
    }

    pub fn state_fingerprint(&self) -> String {
        stable_fingerprint(&self.state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveProgramAbi {
    pub app: String,
    pub mode: LiveProgramMode,
    pub bootstrap: String,
    /// SHA-256 of semantics still executed by generated Rust instead of the live plan.
    pub aot_semantics_digest: String,
    pub named_types: Vec<LiveNamedTypeAbi>,
    pub palette_types: Vec<String>,
    pub extern_structs: Vec<LiveExternStructAbi>,
    pub extern_functions: Vec<LiveExternFunctionAbi>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveProgramMode {
    Application,
    Daemon,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveNamedTypeAbi {
    pub name: String,
    pub variants: Vec<(String, Option<String>)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveExternStructAbi {
    pub name: String,
    pub rust_path: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveExternFunctionAbi {
    pub kind: String,
    pub name: String,
    pub rust_path: String,
    pub params: Vec<(String, String)>,
    pub borrowed: Vec<bool>,
    pub progress: Option<String>,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveStateSchema {
    pub slots: Vec<LiveStateSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LiveStateId {
    pub owner: String,
    pub name: String,
}

impl fmt::Display for LiveStateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.owner, self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveStateSlot {
    pub id: LiveStateId,
    pub ty: String,
    pub storage: LiveStateStorage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveStateStorage {
    App,
    RetainedComponent,
    MountedComponent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveStateChange {
    Added(LiveStateSlot),
    Removed(LiveStateSlot),
    Reinitialized {
        previous: LiveStateSlot,
        next: LiveStateSlot,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveReloadDecision {
    Reload { state_changes: Vec<LiveStateChange> },
    RestartRequired { reasons: Vec<LiveRestartReason> },
}

impl LiveReloadDecision {
    pub fn can_reload(&self) -> bool {
        matches!(self, Self::Reload { .. })
    }
}

/// One compiler-checked live program revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivePlan {
    pub revision: u64,
    pub contract: LiveProgramContract,
    /// A lowered live view. `None` keeps the app's compiled AOT view active.
    pub view: Option<LiveView>,
}

/// The initial renderer-independent live view surface.
///
/// New node families are added here as the live backend reaches AOT parity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LiveView {
    Column {
        key: String,
        children: Vec<LiveView>,
    },
    Row {
        key: String,
        children: Vec<LiveView>,
    },
    Text {
        key: String,
        value: LiveExpression,
    },
    Button {
        key: String,
        label: String,
        disabled: Option<LiveExpression>,
        route: LiveRoute,
    },
    If {
        condition: LiveExpression,
        children: Vec<LiveView>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LiveExpression {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Path(String),
    Unary {
        op: LiveUnaryOp,
        value: Box<LiveExpression>,
    },
    Binary {
        left: Box<LiveExpression>,
        op: LiveBinaryOp,
        right: Box<LiveExpression>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveUnaryOp {
    Not,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveRoute {
    pub handler: String,
    pub args: Vec<LiveExpression>,
}

/// A value copied from generated app state into one live render pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LiveValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
}

/// A live widget event routed back into a compiler-generated AOT handler.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveEvent {
    pub handler: String,
    pub args: Vec<LiveValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveRestartReason {
    ProtocolVersion,
    ProgramIdentity,
    Bootstrap,
    AotSemantics,
    NamedTypes,
    PaletteTypes,
    ExternStructs,
    ExternFunctions,
}

impl fmt::Display for LiveRestartReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProtocolVersion => "the live protocol version changed",
            Self::ProgramIdentity => "the app name or app mode changed",
            Self::Bootstrap => "a process bootstrap setting changed",
            Self::AotSemantics => "behavior still owned by generated Rust changed",
            Self::NamedTypes => "a generated enum changed",
            Self::PaletteTypes => "the generated palette type changed",
            Self::ExternStructs => "an extern struct boundary changed",
            Self::ExternFunctions => "an extern function boundary changed",
        })
    }
}

/// Classifies a checked source-graph change before a running app accepts it.
///
/// Ice-owned state is reconciled by semantic owner/name identity. Compile-time
/// Rust boundaries require a process restart when they change.
pub fn evaluate_live_reload(
    previous: &LiveProgramContract,
    next: &LiveProgramContract,
) -> LiveReloadDecision {
    let mut reasons = Vec::new();
    if previous.protocol_version != next.protocol_version {
        reasons.push(LiveRestartReason::ProtocolVersion);
    }
    if previous.abi.app != next.abi.app || previous.abi.mode != next.abi.mode {
        reasons.push(LiveRestartReason::ProgramIdentity);
    }
    for (changed, reason) in [
        (
            previous.abi.bootstrap != next.abi.bootstrap,
            LiveRestartReason::Bootstrap,
        ),
        (
            previous.abi.aot_semantics_digest != next.abi.aot_semantics_digest,
            LiveRestartReason::AotSemantics,
        ),
        (
            previous.abi.named_types != next.abi.named_types,
            LiveRestartReason::NamedTypes,
        ),
        (
            previous.abi.palette_types != next.abi.palette_types,
            LiveRestartReason::PaletteTypes,
        ),
        (
            previous.abi.extern_structs != next.abi.extern_structs,
            LiveRestartReason::ExternStructs,
        ),
        (
            previous.abi.extern_functions != next.abi.extern_functions,
            LiveRestartReason::ExternFunctions,
        ),
    ] {
        if changed {
            reasons.push(reason);
        }
    }
    if !reasons.is_empty() {
        return LiveReloadDecision::RestartRequired { reasons };
    }

    LiveReloadDecision::Reload {
        state_changes: state_changes(&previous.state, &next.state),
    }
}

fn state_changes(previous: &LiveStateSchema, next: &LiveStateSchema) -> Vec<LiveStateChange> {
    let previous = previous
        .slots
        .iter()
        .map(|slot| (slot.id.clone(), slot))
        .collect::<BTreeMap<_, _>>();
    let next = next
        .slots
        .iter()
        .map(|slot| (slot.id.clone(), slot))
        .collect::<BTreeMap<_, _>>();
    let ids = previous
        .keys()
        .chain(next.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    for id in ids {
        match (previous.get(&id), next.get(&id)) {
            (None, Some(next)) => changes.push(LiveStateChange::Added((*next).clone())),
            (Some(previous), None) => changes.push(LiveStateChange::Removed((*previous).clone())),
            (Some(previous), Some(next)) if previous != next => {
                changes.push(LiveStateChange::Reinitialized {
                    previous: (*previous).clone(),
                    next: (*next).clone(),
                });
            }
            _ => {}
        }
    }
    changes
}

fn stable_fingerprint(value: &impl fmt::Debug) -> String {
    let canonical = format!("{value:?}");
    let first = fnv1a(canonical.as_bytes(), 0xcbf29ce484222325);
    let second = fnv1a(canonical.as_bytes(), 0x84222325cbf29ce4);
    format!("{first:016x}{second:016x}")
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_round_trips_through_serde() {
        let contract = LiveProgramContract {
            protocol_version: LIVE_PROTOCOL_VERSION,
            abi: LiveProgramAbi {
                app: "Demo".into(),
                mode: LiveProgramMode::Application,
                bootstrap: String::new(),
                aot_semantics_digest: String::new(),
                named_types: Vec::new(),
                palette_types: vec!["AppTheme[app]".into()],
                extern_structs: Vec::new(),
                extern_functions: Vec::new(),
            },
            state: LiveStateSchema {
                slots: vec![LiveStateSlot {
                    id: LiveStateId {
                        owner: "app".into(),
                        name: "count".into(),
                    },
                    ty: "i64".into(),
                    storage: LiveStateStorage::App,
                }],
            },
        };

        let encoded = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serde_json::from_str::<LiveProgramContract>(&encoded).unwrap(),
            contract
        );
    }
}
