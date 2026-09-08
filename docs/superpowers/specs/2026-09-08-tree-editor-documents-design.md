# Revisioned editor documents

Status: proposed for root review; no implementation/build yet.
Base: origin/main 480a5290. Integration base must include the final transaction
lane (#4) and protocol manifest work. Issue: #1014.

## Current boundaries inspected

The live #4 worktree is being edited, including temporary Red mutations. This
design uses its declared contract and structural APIs, not temporary guard values.

- wire/editor_transaction.rs: EditorKeyRequest.state is the complete EditorState;
  Commit.before/after repeat full text. TransactionId already includes instance,
  logical document, reset, sequence, attempt, text_revision and observation revision.
- core/codegen/view/tree.rs:1722–1897 computes logical identity from resolved app
  state or component instance, then calls __editor.text() for each Node::Editor.
- guest/editor.rs owns EditorState. guest/editor_binding.rs:81 applies accepted
  state before on_event. guest/slots.rs owns per-driver pending replies/routes.
- runtime/view_tree/editor_transactions.rs:28 owns the existing Lane FIFO;
  Control:246 retains revisions, pending request and committed acknowledgment.
  Requests:630 and commits:776 currently copy full native Content text.
- runtime/view_tree.rs:190 owns EditorField per widget, so shared logical bindings
  still duplicate native Content. Keep widget focus/scroll separate from document.
- wire/snapshot.rs:7 has an 8 MiB aggregate bound; guest/snapshot.rs rejects pending
  transactions and restores into an independent driver without boot.
- Ducktape origin/dev 58372599d, crates/views/pages/src/editor.rs:100–101 currently
  permits 200 history snapshots / 16 MiB. That exceeds the whole snapshot budget.

## Limits and ownership

Proposed hard framework bounds (root approval required):

| Resource | Bound per guest instance |
| --- | --- |
| Complete document | 1,048,576 UTF-8 bytes |
| Logical documents | 16; combined live text <= 4 MiB |
| One raw transfer chunk | 65,536 bytes |
| One full assignment/resync | ceil(length / 65,536), at most 16 chunks |
| Concurrent transfer staging | one 1 MiB receiver buffer; pending descriptors only |
| Per-frame document payload | one chunk, separate from display/picture budgets |
| One atomic patch transaction | <= 256 ranges; inserted bytes and result <= 1 MiB |
| Pending patch staging | one transaction per document; aggregate <= 1 MiB |
| Input admission queue | existing 128 inputs; payload <= 1 MiB aggregate |
| Snapshot | existing exact serialized aggregate <= 8 MiB |

Keep the 64 KiB display budget unchanged. Document names remain bounded at 1,024
bytes. Count documents and reserve the combined live/staging budget before large
allocation; reserve replacement net growth before mutation. Payload decoders
reject advertised over-limit lengths before constructing Vec/String. Frame-wide
checks run before adopting references, patches or transfer data.

Guest Editor remains the one owned application mirror. Do not add another full
text map in slots or change Clone into aliasing Rc mutation. Slots retains only
transfer progress, acknowledgments and routes. Host stores one Content per
logical document; each widget references that Content while retaining its own
native focus/scroll state. The existing logical transaction lane is reused.

History policy remains application-owned. Proposal to consumer: <= 2 MiB total
history serialized bytes, using inverse/forward patches where useful; no host
history or grouping. This is not a new claim that arbitrary application state
fits 8 MiB. Snapshot validates the final complete encoding and fails atomically
when other state consumes the remainder. Never silently trim history at snapshot.
The 64 MiB Wasm memory limiter remains a final sandbox bound; fixture measurements
must include live mirrors, staging, before/after patch temporaries and snapshot
encoding, not just text payload lengths.

## Reference and chunk protocol

Replace Node::Editor text/cursor/reset/revision and EditorOptions.document with
one Node::Editor.document: EditorDocumentRef:

```
EditorDocumentRef { document: String, reset: u64, text_revision: u64,
                    revision: u64, cursor: EditorCursor, byte_len: u32 }
EditorTransferId { instance: u64, document: String, reset: u64, serial: u64 }
EditorTransfer:
  Begin { id, target: EditorDocumentRef }
  Chunk { id, index: u8, bytes: Vec<u8> }
  Complete { id }
  Abort { id }
```

The host mints transfer serials using the existing per-instance monotonic sequence
source. The exact expected target is recorded before requesting bytes. A Node is
not permission to install arbitrary transfer data: every response must match the
pending id and target. Existing binding identity is reused; no text-based alias.

Initial assignment and explicit reset are host-pulled: adopt a new reference,
request the missing version through the generated editor state route, then guest
frames send Begin/chunks/Complete. This also gives the guest the host instance id
before it emits data. Multiple widgets issue only one request. Repeated matching
references send zero document bytes, including a full-tree resync with retained
host document state. A missing reference never opens an editable empty prefix.

One generic bounded byte assembler validates total length, expected chunk count,
contiguous index, exact full-chunk sizes except final, and checked byte accounting.
Bytes may cut through UTF-8. Only Complete with exact length/count performs UTF-8
and final cursor/grapheme validation. Then construct/replace Content or mirror
atomically. Empty documents use Begin/Complete and no data chunks. No incremental
prefix is published. Pending transfers are scheduled serially, without duplicating
source text into a queue; only the affected document's input is suspended.

A wrong instance/reset/serial cannot abort or mutate another active transfer.
Malformed data for the active transfer aborts staging and reports a typed fault.
Interrupted/aborted/stale/oversized/invalid-UTF-8 transfers preserve the last
committed document. After failed reset its old bytes remain present but inputs
stay suspended while the displayed reference requests a different epoch; do not
route new keys to the retired document. Explicit retry gets a fresh serial.

## Incremental input and mirror delivery

Wire EditorKeyRequest removes state.text: retain TransactionId, cursor, key,
repeat and original input_time_ms. Wire Commit replaces before/after text with
validated patches, before/after cursor and after text/observation revisions;
retain kind/history/time. Noop/caret-only commits carry no patches and do not
create history entries. Fault/Cancelled carry metadata, not copied documents.

Route requests through the existing generated mutable Editor delivery path,
rather than invoking decide directly before reaching application state. Validate
identity and mirror revisions first. The guest-local callback gets a borrowed
EditorStateView containing the mirror's text plus cursor/revisions; only the
transport type is text-free. Use a retained typed borrowed callback adapter,
not Any boxes containing borrowed state or an extra persistent text clone.

For Commit, validate the entire patch batch against the mirror, construct the
new value, then accept it and invoke on_event with local borrowed before/after
views. This preserves the existing accept-before-on_event contract and supplies
History the old content without transmitting it. Release temporaries after the
callback; queue acknowledgment only after the authored reducer/claims settle.
History changes made while preparing Undo/Redo must be provisional until the
matching Commit: retries cannot pop another undo entry, and Fault/Cancelled must
discard the provisional transition. The guest owns that transaction-aware policy.
All editors use this path, including ordinary editors without custom key claims;
remove the full-text Event::Edit path rather than preserving it as fallback.

Native edits derive their minimal changed span from retained pre/post Content,
extending endpoints to native-representable grapheme and paired-newline
boundaries. This may scan one document locally; it must not send the whole
document for typing/caret/paste when a smaller change exists. Guest structural
patches remain the admitted batch. Large paste/undo replacements may legitimately
contain up to 1 MiB of changed bytes; charge the dedicated patch budget, not display
text. Do not call a full-document replacement incremental for an ordinary key.

Reuse Lane Ready/Decision/AwaitingGuest/Faulted and its commit barrier. A pending
transfer is a transport prerequisite held by Control, not a second input FIFO or
independent decision scheduler. Native fallback remains one native editor action
only after a matching DefaultEditorAction reply. Reset cancels the pending key.

On same-document mirror mismatch, suspend the existing admitted key and pull the
current host document to the guest through the same chunk assembler. Host input
for that document remains held, while unrelated editors render and run. Atomic
mirror install/ack precedes retry with the same sequence and incremented attempt.
Retain #4's retry/deadline bounds; stale transfer or mismatch never means fallback.

## Restore, instance isolation and external props

A guest snapshot is allowed only when decisions, commit acknowledgments and
transfers have settled. Serialize each owned Editor once with full text/cursor/
revisions; UI references are not another snapshot copy. Pending byte buffers are
not snapshotted. Restore performs the same document/aggregate validation without
init; failure preserves the current instance and assets.

A replacement guest starts a fresh document-session identity, even when its host
Surface and native Inputs are retained. Reject queued old-session chunks/replies.
Candidate preparation must complete its referenced document transfers before
claiming a fully validated first frame or installing the new instance. Up to
16 bounded local ticks per full document may be needed; the old guest remains
installed throughout. Do not claim a reference-only first frame is complete.
No automatic old-session byte aliasing or duplicate decoder is introduced.

Ducktape must deliver a 1 MiB document as <=64 KiB raw chunks plus metadata, not a
single ordinary 1 MiB props value. The guest can use a small public EditorLoad
builder (push indexed bytes, finish -> Result<Editor>) backed by the same bounded
assembler and assign the completed value using normal reset semantics. Application
RPC/connection generations are checked by the application before pushing data;
this framework does not fetch documents, interpret Pages blocks or sign/save them.
Existing page-block 768 KiB/page-query 6 MiB limits need consumer-side pagination
and block serialization; a 1 MiB editor does not enlarge those RPC limits.

## Evidence and implementation order

1. Wire document limits/reference/transfer and pure atomic assembler; Red for
   publishing a prefix, accepting stale id/order and invalid assembled UTF-8.
2. Guest mirror/delivery adapter and host logical Content registry, reusing the
   lane. Red for duplicated unchanged bytes and stale commit application.
3. Core projection and no-init snapshot/restore; update both native/Wasm fixtures
   and first-frame completeness integration, then downstream Pages wiring.
4. Actual native/Wasm: exact 1 MiB Unicode crossing chunk boundaries, load/edit/save
   equality, duplicate-widget + unchanged redraw payload=0, surrounding exhausted
   display budget leaves the document intact; typing/paste/structural/undo/redo
   wire payload equals changes rather than full before/after documents.
5. Interrupt every chunk boundary, duplicate/out-of-order/over-limit/wrong-session
   transfer, reset and same-document resync/retry; previous committed state survives.
6. Hot reload with document/cursor/history under 8 MiB, oversized snapshot/candidate
   rejection preserves old instance/assets. Include long single-line and multiline
   documents to measure actual retained editor layout/memory under the 64 MiB guest
   limit. A byte-only test is not evidence that a 1 MiB editor renders acceptably.

No syntax keyword, host RPC/business logic, painting/highlighter or caret overlay
is added. Wire shape changes require the protocol epoch update. Implementation
starts only after root reviews these limits and callback/restore contracts.
