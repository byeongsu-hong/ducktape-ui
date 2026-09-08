//! Complete owned guest state. Unlike a rendered tree, it must never be truncated.
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

pub const MAX_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
const MAX_DEPTH: usize = 32;
const MAX_VALUES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: String,
    pub state: SnapshotValue,
}

impl Snapshot {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        if crate::encoded_size(self) > MAX_SNAPSHOT_BYTES as u64 {
            return Err("snapshot exceeds the byte budget".into());
        }
        Ok(crate::encode(self))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err("snapshot exceeds the byte budget".into());
        }
        let snapshot: Self = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_limit(MAX_SNAPSHOT_BYTES as u64)
            .reject_trailing_bytes()
            .deserialize(bytes)
            .map_err(|error| error.to_string())?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema.len() != 64 || !self.schema.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("snapshot schema must be a SHA-256 identifier".into());
        }
        let mut values = MAX_VALUES;
        let mut bytes = MAX_SNAPSHOT_BYTES - self.schema.len();
        self.state.validate(0, &mut values, &mut bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum SnapshotValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<SnapshotValue>),
    Option(Option<Box<SnapshotValue>>),
    Record {
        name: String,
        fields: Vec<(String, SnapshotValue)>,
    },
}

impl SnapshotValue {
    fn validate(&self, depth: usize, values: &mut usize, bytes: &mut usize) -> Result<(), String> {
        if depth >= MAX_DEPTH || *values == 0 {
            return Err("snapshot value budget exceeded".into());
        }
        *values -= 1;
        let charge = |remaining: &mut usize, count: usize| {
            *remaining = remaining
                .checked_sub(count)
                .ok_or("snapshot byte budget exceeded")?;
            Ok::<_, String>(())
        };
        match self {
            Self::F64(value) if !value.is_finite() => Err("snapshot number must be finite".into()),
            Self::Str(value) => charge(bytes, value.len()),
            Self::Bytes(value) => charge(bytes, value.len()),
            Self::List(items) => {
                for item in items {
                    item.validate(depth + 1, values, bytes)?;
                }
                Ok(())
            }
            Self::Option(Some(value)) => value.validate(depth + 1, values, bytes),
            Self::Record { name, fields } => {
                charge(bytes, name.len())?;
                for (name, value) in fields {
                    charge(bytes, name.len())?;
                    value.validate(depth + 1, values, bytes)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static VALUES: Cell<usize> = const { Cell::new(0) };
}

impl<'de> Deserialize<'de> for SnapshotValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if DEPTH.get() == 0 {
            VALUES.set(0);
        }
        if DEPTH.get() >= MAX_DEPTH || VALUES.get() >= MAX_VALUES {
            return Err(serde::de::Error::custom("snapshot value budget exceeded"));
        }
        DEPTH.set(DEPTH.get() + 1);
        VALUES.set(VALUES.get() + 1);
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                DEPTH.set(DEPTH.get() - 1);
            }
        }
        let _guard = Guard;
        // This private enum mirrors serialization, routing recursion through
        // the guarded decoder before allocating children.
        #[derive(Deserialize)]
        enum Value {
            Unit,
            Bool(bool),
            I64(i64),
            F64(f64),
            Str(String),
            Bytes(Vec<u8>),
            List(#[serde(deserialize_with = "decode_values")] Vec<SnapshotValue>),
            Option(Option<Box<SnapshotValue>>),
            Record {
                name: String,
                #[serde(deserialize_with = "decode_values")]
                fields: Vec<(String, SnapshotValue)>,
            },
        }
        Ok(match Value::deserialize(deserializer)? {
            Value::Unit => Self::Unit,
            Value::Bool(v) => Self::Bool(v),
            Value::I64(v) => Self::I64(v),
            Value::F64(v) => Self::F64(v),
            Value::Str(v) => Self::Str(v),
            Value::Bytes(v) => Self::Bytes(v),
            Value::List(v) => Self::List(v),
            Value::Option(v) => Self::Option(v),
            Value::Record { name, fields } => Self::Record { name, fields },
        })
    }
}

fn decode_values<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Values<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Values<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded snapshot values")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            if seq
                .size_hint()
                .is_some_and(|len| len > MAX_VALUES.saturating_sub(VALUES.get()))
            {
                return Err(serde::de::Error::custom(
                    "snapshot collection budget exceeded",
                ));
            }
            let mut values = Vec::new();
            while let Some(value) = seq.next_element()? {
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Values(std::marker::PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(state: SnapshotValue) -> Snapshot {
        Snapshot {
            schema: "a".repeat(64),
            state,
        }
    }
    #[test]
    fn complete_state_round_trips_and_rejects_trailing_data() {
        let expected = snapshot(SnapshotValue::Record {
            name: "App".into(),
            fields: vec![
                ("draft".into(), SnapshotValue::Str("계속 편집".into())),
                ("bytes".into(), SnapshotValue::Bytes(vec![0, 128, 255])),
                (
                    "child".into(),
                    SnapshotValue::Option(Some(Box::new(SnapshotValue::I64(7)))),
                ),
            ],
        });
        let mut bytes = expected.encode().unwrap();
        assert_eq!(Snapshot::decode(&bytes).unwrap(), expected);
        bytes.push(0);
        assert!(
            Snapshot::decode(&bytes).is_err(),
            "trailing state must not be silently ignored"
        );
    }
    #[test]
    fn oversized_invalid_and_deep_state_is_refused_whole() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(snapshot(SnapshotValue::F64(value)).encode().is_err());
        }
        assert!(
            snapshot(SnapshotValue::Bytes(vec![0; MAX_SNAPSHOT_BYTES]))
                .encode()
                .is_err()
        );
        let mut deep = SnapshotValue::Unit;
        for _ in 0..MAX_DEPTH {
            deep = SnapshotValue::Option(Some(Box::new(deep)));
        }
        let deep = snapshot(deep);
        assert!(deep.encode().is_err());
        assert!(Snapshot::decode(&crate::encode(&deep)).is_err());
        let mut bomb = snapshot(SnapshotValue::List(vec![])).encode().unwrap();
        // Fixed-width schema length + schema bytes + enum tag, then list length.
        bomb[8 + 64 + 4..8 + 64 + 4 + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(Snapshot::decode(&bomb).is_err());
        let small = snapshot(SnapshotValue::Unit);
        assert_eq!(
            Snapshot::decode(&small.encode().unwrap()).unwrap(),
            small,
            "failed decoding releases the depth budget"
        );
    }
}
