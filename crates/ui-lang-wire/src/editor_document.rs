//! Revisioned document transfer, independent of display text and native layout.
use serde::{Deserialize, Serialize};

use crate::EditorCursor;

pub const MAX_EDITOR_DOCUMENT_BYTES: usize = 1_048_576;
pub const MAX_EDITOR_CHUNK_BYTES: usize = 65_536;
pub const MAX_EDITOR_CHUNKS: usize = MAX_EDITOR_DOCUMENT_BYTES / MAX_EDITOR_CHUNK_BYTES;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorDocumentRef {
    #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
    pub document: String,
    pub reset: u64,
    pub text_revision: u64,
    pub revision: u64,
    pub cursor: EditorCursor,
    pub byte_len: u32,
}

impl EditorDocumentRef {
    pub fn validate(&self) -> Result<(), EditorTransferError> {
        if self.document.is_empty()
            || self.document.len() > 1024
            || self.byte_len as usize > MAX_EDITOR_DOCUMENT_BYTES
        {
            return Err(EditorTransferError::Limit);
        }
        for position in std::iter::once(self.cursor.position).chain(self.cursor.selection) {
            if position.line > self.byte_len || position.column > self.byte_len {
                return Err(EditorTransferError::Cursor);
            }
        }
        Ok(())
    }

    pub fn validate_text(&self, text: &str) -> Result<(), EditorTransferError> {
        self.validate()?;
        if text.len() != self.byte_len as usize {
            return Err(EditorTransferError::Length);
        }
        let mut cursor = self.cursor;
        cursor.clamp(text);
        if cursor != self.cursor {
            return Err(EditorTransferError::Cursor);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorTransferId {
    pub instance: u64,
    #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
    pub document: String,
    pub reset: u64,
    pub serial: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTransfer {
    Begin {
        id: EditorTransferId,
        target: EditorDocumentRef,
    },
    Chunk {
        id: EditorTransferId,
        index: u8,
        #[serde(deserialize_with = "decode_chunk")]
        bytes: Vec<u8>,
    },
    Complete {
        id: EditorTransferId,
    },
    Abort {
        id: EditorTransferId,
    },
}

impl EditorTransfer {
    pub fn id(&self) -> &EditorTransferId {
        match self {
            Self::Begin { id, .. }
            | Self::Chunk { id, .. }
            | Self::Complete { id }
            | Self::Abort { id } => id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTransferError {
    Limit,
    Identity,
    Order,
    Length,
    Utf8,
    Cursor,
    Aborted,
}

/// One bounded byte buffer, also usable by application-owned document loading.
/// No partial string can be observed. UTF-8 may cross any raw chunk boundary.
#[derive(Debug)]
pub struct EditorChunkAssembler {
    expected: usize,
    next: usize,
    bytes: Vec<u8>,
}

impl EditorChunkAssembler {
    pub fn new(byte_len: usize) -> Result<Self, EditorTransferError> {
        if byte_len > MAX_EDITOR_DOCUMENT_BYTES {
            return Err(EditorTransferError::Limit);
        }
        Ok(Self {
            expected: byte_len,
            next: 0,
            bytes: Vec::with_capacity(byte_len),
        })
    }

    pub fn push(&mut self, index: u8, bytes: &[u8]) -> Result<(), EditorTransferError> {
        if usize::from(index) != self.next || self.bytes.len() == self.expected {
            return Err(EditorTransferError::Order);
        }
        let expected = (self.expected - self.bytes.len()).min(MAX_EDITOR_CHUNK_BYTES);
        if bytes.len() != expected {
            return Err(EditorTransferError::Length);
        }
        self.bytes.extend_from_slice(bytes);
        self.next += 1;
        Ok(())
    }

    pub fn buffered_bytes(&self) -> usize {
        self.bytes.len()
    }

    pub fn finish(self) -> Result<String, EditorTransferError> {
        if self.bytes.len() != self.expected
            || self.next != self.expected.div_ceil(MAX_EDITOR_CHUNK_BYTES)
        {
            return Err(EditorTransferError::Length);
        }
        String::from_utf8(self.bytes).map_err(|_| EditorTransferError::Utf8)
    }
}

/// A receiver is created only for an explicitly requested id and reference.
/// Wrong identities cannot discard its buffer. Malformed active transfers end
/// this receiver; an explicit retry must construct one with a new serial.
#[derive(Debug)]
pub struct EditorTransferReceiver {
    id: EditorTransferId,
    target: EditorDocumentRef,
    assembler: Option<EditorChunkAssembler>,
    ended: bool,
}

impl EditorTransferReceiver {
    pub fn new(
        id: EditorTransferId,
        target: EditorDocumentRef,
    ) -> Result<Self, EditorTransferError> {
        target.validate()?;
        if id.document != target.document || id.reset != target.reset {
            return Err(EditorTransferError::Identity);
        }
        Ok(Self {
            id,
            target,
            assembler: None,
            ended: false,
        })
    }

    pub fn buffered_bytes(&self) -> usize {
        self.assembler
            .as_ref()
            .map_or(0, EditorChunkAssembler::buffered_bytes)
    }

    pub fn receive(
        &mut self,
        transfer: &EditorTransfer,
    ) -> Result<Option<String>, EditorTransferError> {
        if transfer.id() != &self.id {
            return Err(EditorTransferError::Identity);
        }
        if self.ended {
            return Err(EditorTransferError::Order);
        }
        let result = self.receive_current(transfer);
        if result.is_err() {
            self.assembler = None;
            self.ended = true;
        }
        result
    }

    fn receive_current(
        &mut self,
        transfer: &EditorTransfer,
    ) -> Result<Option<String>, EditorTransferError> {
        match transfer {
            EditorTransfer::Begin { target, .. } => {
                if target != &self.target {
                    return Err(EditorTransferError::Identity);
                }
                if self.assembler.is_some() {
                    return Err(EditorTransferError::Order);
                }
                self.assembler = Some(EditorChunkAssembler::new(target.byte_len as usize)?);
                Ok(None)
            }
            EditorTransfer::Chunk { index, bytes, .. } => {
                self.assembler
                    .as_mut()
                    .ok_or(EditorTransferError::Order)?
                    .push(*index, bytes)?;
                Ok(None)
            }
            EditorTransfer::Complete { .. } => {
                self.ended = true;
                let text = self
                    .assembler
                    .take()
                    .ok_or(EditorTransferError::Order)?
                    .finish()?;
                self.target.validate_text(&text)?;
                Ok(Some(text))
            }
            EditorTransfer::Abort { .. } => Err(EditorTransferError::Aborted),
        }
    }
}

fn decode_chunk<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct Chunk;
    impl<'de> serde::de::Visitor<'de> for Chunk {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 64 KiB of raw editor bytes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
            if seq.size_hint().is_some_and(|n| n > MAX_EDITOR_CHUNK_BYTES) {
                return Err(serde::de::Error::custom("editor chunk byte limit"));
            }
            let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(byte) = seq.next_element()? {
                if bytes.len() == MAX_EDITOR_CHUNK_BYTES {
                    return Err(serde::de::Error::custom("editor chunk byte limit"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    d.deserialize_seq(Chunk)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(len: usize) -> (EditorTransferId, EditorDocumentRef) {
        (
            EditorTransferId {
                instance: 3,
                document: "app:draft".into(),
                reset: 7,
                serial: 11,
            },
            EditorDocumentRef {
                document: "app:draft".into(),
                reset: 7,
                text_revision: 2,
                revision: 4,
                cursor: EditorCursor::default(),
                byte_len: len as u32,
            },
        )
    }

    fn begun(len: usize) -> (EditorTransferId, EditorTransferReceiver) {
        let (id, target) = metadata(len);
        let mut receiver = EditorTransferReceiver::new(id.clone(), target.clone()).unwrap();
        assert_eq!(
            receiver.receive(&EditorTransfer::Begin {
                id: id.clone(),
                target
            }),
            Ok(None)
        );
        (id, receiver)
    }

    #[test]
    fn exact_one_mib_is_published_only_after_complete_even_when_utf8_crosses_a_chunk() {
        let mut text = "x".repeat(MAX_EDITOR_DOCUMENT_BYTES - 3);
        text.insert(MAX_EDITOR_CHUNK_BYTES - 1, '한');
        let (id, mut receiver) = begun(text.len());
        let chunks: Vec<_> = text.as_bytes().chunks(MAX_EDITOR_CHUNK_BYTES).collect();
        assert_eq!(chunks.len(), MAX_EDITOR_CHUNKS);
        assert!(std::str::from_utf8(chunks[0]).is_err());
        for (index, bytes) in chunks.iter().enumerate() {
            assert!(
                matches!(
                    receiver.receive(&EditorTransfer::Chunk {
                        id: id.clone(),
                        index: index as u8,
                        bytes: bytes.to_vec(),
                    }),
                    Ok(None)
                ),
                "a chunk must not publish a document prefix"
            );
            assert_eq!(
                receiver.buffered_bytes(),
                (index + 1) * MAX_EDITOR_CHUNK_BYTES
            );
        }
        assert_eq!(
            receiver.receive(&EditorTransfer::Complete { id }),
            Ok(Some(text))
        );
        assert_eq!(receiver.buffered_bytes(), 0);
    }

    #[test]
    fn every_interruption_boundary_discards_staging_without_publishing_a_prefix() {
        for boundary in 0..=MAX_EDITOR_CHUNKS {
            let (id, mut receiver) = begun(MAX_EDITOR_DOCUMENT_BYTES);
            for index in 0..boundary {
                assert_eq!(
                    receiver.receive(&EditorTransfer::Chunk {
                        id: id.clone(),
                        index: index as u8,
                        bytes: vec![b'x'; MAX_EDITOR_CHUNK_BYTES],
                    }),
                    Ok(None)
                );
            }
            assert_eq!(
                receiver.receive(&EditorTransfer::Abort { id: id.clone() }),
                Err(EditorTransferError::Aborted)
            );
            assert_eq!(receiver.buffered_bytes(), 0);
            assert_eq!(
                receiver.receive(&EditorTransfer::Complete { id }),
                Err(EditorTransferError::Order)
            );
        }
    }

    #[test]
    fn stale_identity_cannot_abort_or_append_to_the_requested_document() {
        let (id, mut receiver) = begun(MAX_EDITOR_CHUNK_BYTES + 1);
        assert_eq!(
            receiver.receive(&EditorTransfer::Chunk {
                id: id.clone(),
                index: 0,
                bytes: vec![b'a'; MAX_EDITOR_CHUNK_BYTES],
            }),
            Ok(None)
        );
        for change in 0..4 {
            let mut stale = id.clone();
            match change {
                0 => stale.instance += 1,
                1 => stale.serial += 1,
                2 => stale.reset += 1,
                _ => stale.document = "app:another".into(),
            }
            for event in [
                EditorTransfer::Abort { id: stale.clone() },
                EditorTransfer::Chunk {
                    id: stale.clone(),
                    index: 1,
                    bytes: vec![b'b'],
                },
                EditorTransfer::Complete { id: stale },
            ] {
                assert_eq!(receiver.receive(&event), Err(EditorTransferError::Identity));
                assert_eq!(receiver.buffered_bytes(), MAX_EDITOR_CHUNK_BYTES);
            }
        }
        receiver
            .receive(&EditorTransfer::Chunk {
                id: id.clone(),
                index: 1,
                bytes: vec![b'b'],
            })
            .unwrap();
        let text = receiver
            .receive(&EditorTransfer::Complete { id })
            .unwrap()
            .unwrap();
        assert_eq!(text, format!("{}b", "a".repeat(MAX_EDITOR_CHUNK_BYTES)));
    }

    #[test]
    fn malformed_active_transfer_fails_closed_instead_of_becoming_a_partial_document() {
        for bad in 0..4 {
            let (id, mut receiver) = begun(MAX_EDITOR_CHUNK_BYTES + 1);
            receiver
                .receive(&EditorTransfer::Chunk {
                    id: id.clone(),
                    index: 0,
                    bytes: vec![b'a'; MAX_EDITOR_CHUNK_BYTES],
                })
                .unwrap();
            let event = match bad {
                0 => EditorTransfer::Chunk {
                    id: id.clone(),
                    index: 0,
                    bytes: vec![b'a'; MAX_EDITOR_CHUNK_BYTES],
                },
                1 => EditorTransfer::Chunk {
                    id: id.clone(),
                    index: 2,
                    bytes: vec![b'b'],
                },
                2 => EditorTransfer::Chunk {
                    id: id.clone(),
                    index: 1,
                    bytes: vec![b'b'; 2],
                },
                _ => EditorTransfer::Complete { id: id.clone() },
            };
            assert!(receiver.receive(&event).is_err());
            assert_eq!(receiver.buffered_bytes(), 0);
            assert_eq!(
                receiver.receive(&EditorTransfer::Complete { id }),
                Err(EditorTransferError::Order)
            );
        }
    }

    #[test]
    fn complete_checks_utf8_and_native_cursor_and_empty_documents_need_no_chunk() {
        let (id, mut receiver) = begun(2);
        receiver
            .receive(&EditorTransfer::Chunk {
                id: id.clone(),
                index: 0,
                bytes: vec![0xff, 0xff],
            })
            .unwrap();
        assert_eq!(
            receiver.receive(&EditorTransfer::Complete { id }),
            Err(EditorTransferError::Utf8)
        );
        let (id, mut target) = metadata(3);
        target.cursor.position.column = 1;
        let mut receiver = EditorTransferReceiver::new(id.clone(), target.clone()).unwrap();
        receiver
            .receive(&EditorTransfer::Begin {
                id: id.clone(),
                target,
            })
            .unwrap();
        receiver
            .receive(&EditorTransfer::Chunk {
                id: id.clone(),
                index: 0,
                bytes: "e\u{301}".as_bytes().to_vec(),
            })
            .unwrap();
        assert_eq!(
            receiver.receive(&EditorTransfer::Complete { id }),
            Err(EditorTransferError::Cursor)
        );
        let (id, mut receiver) = begun(0);
        assert_eq!(
            receiver.receive(&EditorTransfer::Complete { id }),
            Ok(Some(String::new()))
        );
    }

    #[test]
    fn reference_limits_are_checked_before_allocating_a_document() {
        let (id, mut target) = metadata(MAX_EDITOR_DOCUMENT_BYTES + 1);
        assert_eq!(
            EditorTransferReceiver::new(id.clone(), target.clone()).unwrap_err(),
            EditorTransferError::Limit
        );
        target.byte_len = 0;
        target.document = "d".repeat(1025);
        assert_eq!(target.validate(), Err(EditorTransferError::Limit));
        let (_, mut target) = metadata(0);
        target.cursor.position.line = 1;
        assert_eq!(target.validate(), Err(EditorTransferError::Cursor));
        target.cursor = EditorCursor::default();
        target.reset += 1;
        assert_eq!(
            EditorTransferReceiver::new(id, target).unwrap_err(),
            EditorTransferError::Identity
        );
        assert!(EditorChunkAssembler::new(MAX_EDITOR_DOCUMENT_BYTES + 1).is_err());
    }

    #[test]
    fn decoder_rejects_advertised_oversized_chunks_before_reading_their_payload() {
        let (id, _) = metadata(0);
        let event = EditorTransfer::Chunk {
            id,
            index: 0,
            bytes: vec![],
        };
        let mut encoded = crate::encode(&event);
        let end = encoded.len();
        encoded[end - 8..].copy_from_slice(&((MAX_EDITOR_CHUNK_BYTES + 1) as u64).to_le_bytes());
        let error = crate::decode::<EditorTransfer>(&encoded)
            .unwrap_err()
            .to_string();
        assert!(error.contains("editor chunk byte limit"), "{error}");
    }
}
