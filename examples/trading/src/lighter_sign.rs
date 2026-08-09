//! What Lighter's API key signs: Schnorr over ECgFp5 with a Poseidon2
//! challenge, under a read token and under the L2 transactions that trade.
//!
//! One key does both. The `api_key_index` an account registers is the same
//! index in both preimages, and the curve, the sponge and the signature below
//! are shared — which is why the transactions live here beside the token
//! rather than in the adapter that posts them. What differs is only what is
//! hashed: the token hashes three ASCII fields, a transaction hashes its own
//! fields as Goldilocks elements under the deployment's chain id.
//!
//! **Nothing here submits anything.** This module builds and signs; `lighter.rs`
//! is what reaches `/sendTx`, behind the same gate every read passes.
//!
//! ## The token
//!
//! Lighter gates its private reads behind a token the caller signs itself:
//!
//! ```text
//! {deadline_unix}:{account_index}:{api_key_index}:{signature_hex}
//! ```
//!
//! Only the first three ASCII fields are signed. The route is not in the
//! preimage, so one token opens every gated read — which is also why the
//! deadline is the only thing bounding a leaked token, and why `auth_token`
//! refuses to mint one that outlives the venue's own 8h ceiling.
//!
//! ## Where the curve comes from
//!
//! The curve and its scalar field are `github.com/pornin/ecgfp5`, pinned by
//! revision in `Cargo.toml`. That repository is the reference: Lighter's own
//! open Go signer (`elliottech/poseidon_crypto`) cites it as the design and
//! first implementation, so depending on it puts this module *upstream* of the
//! venue's stack rather than beside it as a third reimplementation. It has no
//! dependencies of its own, is `no_std`, contains no `unsafe`, and is MIT —
//! the same licence as this workspace.
//!
//! A git revision was chosen over the two alternatives. `plonky2_ecgfp5` on
//! crates.io is a *port* of the same reference (one hop further from it, not
//! closer) and drags in the whole `plonky2` proving stack to reach an
//! out-of-circuit curve. Vendoring the ~4500 lines into this tree would buy
//! offline builds, which a `git` dependency gives up: a cold cache now needs
//! GitHub, not just a crates.io mirror, and if that repository ever
//! disappears the fix is to vendor it at that point. `Cargo.lock` pins the
//! exact commit, so what builds is fixed either way.
//!
//! Everything the reference does not cover — Poseidon2, the sponge, the
//! Schnorr layer, the token — is here, and every piece of it is checked
//! against values the venue's own signer produced rather than against itself.
//!
//! ## Poseidon2
//!
//! Transcribed from `poseidon_crypto/hash/poseidon2_goldilocks_plonky2`, which
//! is the plonky2 constant set (width 12, rate 8, 8 external and 22 partial
//! rounds, x^7 s-box). The round constants below are that package's, verbatim.
//! The permutation is written in the same fused order the Go uses — each
//! linear layer carries the *next* round's constants — because the fusion is
//! observable: the first external layer runs before any constant is added, and
//! `EXTERNAL_CONSTANTS[4]` is added by the last partial round rather than by
//! an external one.
//!
//! ## The transactions
//!
//! An order on Lighter is an L2 transaction, not a JSON action: the fields are
//! hashed as field elements in the sequencer's own order, and the body sent
//! beside the signature is the same fields as JSON. Both are built together at
//! the bottom of this file, from one set of values, for the reason `signing.rs`
//! builds Hyperliquid's action once and packs it twice — a field signed in one
//! shape and sent in another recovers a stranger.
//!
//! The layouts are transcribed from `elliottech/lighter-go`'s
//! `L2CreateOrderTxInfo.Hash` and `L2CancelOrderTxInfo.Hash`, and every claim
//! about them is pinned against digests and bodies that the venue's own signer
//! produced for the same inputs — the same `.so` the token's vectors came
//! from, so nothing here is documentation taken on trust.

// Complete and held to its evidence, but nothing points at it until the ticket
// is wired to the order path.
#![allow(dead_code)]

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ecgfp5::curve::Point;
use ecgfp5::field::{GFp, GFp5};
use ecgfp5::scalar::Scalar;

use crate::lighter::Zone;

// ---------------------------------------------------------------------------
// Poseidon2 over Goldilocks, plonky2 variant.

const WIDTH: usize = 12;
const RATE: usize = 8;
const ROUNDS_P: usize = 22;

const fn gfps<const N: usize>(values: [u64; N]) -> [GFp; N] {
    let mut out = [GFp::ZERO; N];
    let mut i = 0;
    while i < N {
        out[i] = GFp::from_u64_reduce(values[i]);
        i += 1;
    }
    out
}

const EXTERNAL_CONSTANTS: [[GFp; 12]; 8] = [
    gfps([
        15492826721047263190,
        11728330187201910315,
        8836021247773420868,
        16777404051263952451,
        5510875212538051896,
        6173089941271892285,
        2927757366422211339,
        10340958981325008808,
        8541987352684552425,
        9739599543776434497,
        15073950188101532019,
        12084856431752384512,
    ]),
    gfps([
        4584713381960671270,
        8807052963476652830,
        54136601502601741,
        4872702333905478703,
        5551030319979516287,
        12889366755535460989,
        16329242193178844328,
        412018088475211848,
        10505784623379650541,
        9758812378619434837,
        7421979329386275117,
        375240370024755551,
    ]),
    gfps([
        3331431125640721931,
        15684937309956309981,
        578521833432107983,
        14379242000670861838,
        17922409828154900976,
        8153494278429192257,
        15904673920630731971,
        11217863998460634216,
        3301540195510742136,
        9937973023749922003,
        3059102938155026419,
        1895288289490976132,
    ]),
    gfps([
        5580912693628927540,
        10064804080494788323,
        9582481583369602410,
        10186259561546797986,
        247426333829703916,
        13193193905461376067,
        6386232593701758044,
        17954717245501896472,
        1531720443376282699,
        2455761864255501970,
        11234429217864304495,
        4746959618548874102,
    ]),
    gfps([
        13571697342473846203,
        17477857865056504753,
        15963032953523553760,
        16033593225279635898,
        14252634232868282405,
        8219748254835277737,
        7459165569491914711,
        15855939513193752003,
        16788866461340278896,
        7102224659693946577,
        3024718005636976471,
        13695468978618890430,
    ]),
    gfps([
        8214202050877825436,
        2670727992739346204,
        16259532062589659211,
        11869922396257088411,
        3179482916972760137,
        13525476046633427808,
        3217337278042947412,
        14494689598654046340,
        15837379330312175383,
        8029037639801151344,
        2153456285263517937,
        8301106462311849241,
    ]),
    gfps([
        13294194396455217955,
        17394768489610594315,
        12847609130464867455,
        14015739446356528640,
        5879251655839607853,
        9747000124977436185,
        8950393546890284269,
        10765765936405694368,
        14695323910334139959,
        16366254691123000864,
        15292774414889043182,
        10910394433429313384,
    ]),
    gfps([
        17253424460214596184,
        3442854447664030446,
        3005570425335613727,
        10859158614900201063,
        9763230642109343539,
        6647722546511515039,
        909012944955815706,
        18101204076790399111,
        11588128829349125809,
        15863878496612806566,
        5201119062417750399,
        176665553780565743,
    ]),
];

const INTERNAL_CONSTANTS: [GFp; ROUNDS_P] = gfps([
    11921381764981422944,
    10318423381711320787,
    8291411502347000766,
    229948027109387563,
    9152521390190983261,
    7129306032690285515,
    15395989607365232011,
    8641397269074305925,
    17256848792241043600,
    6046475228902245682,
    12041608676381094092,
    12785542378683951657,
    14546032085337914034,
    3304199118235116851,
    16499627707072547655,
    10386478025625759321,
    13475579315436919170,
    16042710511297532028,
    1411266850385657080,
    9024840976168649958,
    14047056970978379368,
    838728605080212101,
]);

const MATRIX_DIAG_12: [GFp; WIDTH] = gfps([
    0xc3b6c08e23ba9300,
    0xd84b5de94a324fb6,
    0x0d0c371c5b35b84f,
    0x7964f570e7188037,
    0x5daf18bbd996604b,
    0x6743bc47b9595257,
    0x5528b9362c59bb70,
    0xac45e25b7127b68b,
    0xa2077d7dfbb606b5,
    0xf3faac6faee378ae,
    0x0c6388b51545e883,
    0xd27dbb6944917b60,
]);

/// The s-box, x^7, in four multiplications.
fn pow7(x: GFp) -> GFp {
    let x2 = x.square();
    x * x2 * x2.square()
}

/// The external (full-round) linear layer: the 4x4 MDS matrix over each of the
/// three chunks, then a circulant fold that mixes the chunks together.
fn external_linear_layer(state: &mut [GFp; WIDTH]) {
    for chunk in 0..3 {
        let base = chunk * 4;
        let (v0, v1, v2, v3) = (
            state[base],
            state[base + 1],
            state[base + 2],
            state[base + 3],
        );
        let t01 = v0 + v1;
        let t23 = v2 + v3;
        let t = t01 + t23;
        state[base] = t + t01 + v1;
        state[base + 1] = t + v1 + v2.double();
        state[base + 2] = t + t23 + v3;
        state[base + 3] = t + v3 + v0.double();
    }
    for lane in 0..4 {
        let sum = state[lane] + state[lane + 4] + state[lane + 8];
        state[lane] += sum;
        state[lane + 4] += sum;
        state[lane + 8] += sum;
    }
}

fn external_linear_layer_rc(state: &mut [GFp; WIDTH], rc: &[GFp; WIDTH]) {
    external_linear_layer(state);
    for i in 0..WIDTH {
        state[i] += rc[i];
    }
}

fn sbox(state: &mut [GFp; WIDTH]) {
    for lane in state.iter_mut() {
        *lane = pow7(*lane);
    }
}

/// The partial rounds: the s-box on lane 0 only, then the internal matrix
/// `diag(MATRIX_DIAG_12) + all-ones`. The constant folded in at the end of
/// each round belongs to the *next* round, and the last round folds in
/// `EXTERNAL_CONSTANTS[4]` — the first external round after this block never
/// adds constants of its own.
fn partial_rounds(state: &mut [GFp; WIDTH]) {
    state[0] += INTERNAL_CONSTANTS[0];
    for round in 0..ROUNDS_P {
        let s0 = pow7(state[0]);
        let mut sum = s0;
        for lane in &state[1..] {
            sum += *lane;
        }
        let next = if round + 1 < ROUNDS_P {
            let mut rc = [GFp::ZERO; WIDTH];
            rc[0] = INTERNAL_CONSTANTS[round + 1];
            rc
        } else {
            EXTERNAL_CONSTANTS[4]
        };
        state[0] = s0 * MATRIX_DIAG_12[0] + sum + next[0];
        for i in 1..WIDTH {
            state[i] = state[i] * MATRIX_DIAG_12[i] + sum + next[i];
        }
    }
}

fn permute(state: &mut [GFp; WIDTH]) {
    for rc in &EXTERNAL_CONSTANTS[0..4] {
        external_linear_layer_rc(state, rc);
        sbox(state);
    }
    external_linear_layer(state);
    partial_rounds(state);
    for rc in &EXTERNAL_CONSTANTS[5..8] {
        sbox(state);
        external_linear_layer_rc(state, rc);
    }
    sbox(state);
    external_linear_layer(state);
}

/// The venue's unpadded sponge. There is no padding and no domain separator on
/// the length, so a short final chunk leaves the rest of the rate holding the
/// previous permutation's output rather than zeros — reproduced here rather
/// than corrected, because the digest has to match theirs.
fn hash_n_to_m(input: &[GFp], out: &mut [GFp]) {
    let mut state = [GFp::ZERO; WIDTH];
    for chunk in input.chunks(RATE) {
        state[..chunk.len()].copy_from_slice(chunk);
        permute(&mut state);
    }
    let mut written = 0;
    while written < out.len() {
        if written > 0 {
            permute(&mut state);
        }
        let take = RATE.min(out.len() - written);
        out[written..written + take].copy_from_slice(&state[..take]);
        written += take;
    }
}

/// `HashToQuinticExtension`: squeeze five limbs and read them as one Fp5
/// element, which is the scheme's message space.
fn hash_to_quintic_extension(input: &[GFp]) -> GFp5 {
    let mut out = [GFp::ZERO; 5];
    hash_n_to_m(input, &mut out);
    GFp5(out)
}

// ---------------------------------------------------------------------------
// Schnorr over ECgFp5.

/// A message reduced to the single Fp5 element the scheme actually signs.
///
/// The venue hashes before it signs and transmits only the 40-byte digest to
/// its own signer, so this is the real boundary: everything above it is a
/// message, everything below it is curve arithmetic.
#[derive(Clone, Copy, Debug)]
pub struct Digest(GFp5);

impl Digest {
    /// Reject non-canonical limbs, matching the venue's own
    /// `FromCanonicalLittleEndianBytes`. Two byte strings that differ only by
    /// a multiple of p must not name the same digest.
    pub fn from_bytes(bytes: &[u8; 40]) -> Option<Self> {
        let (element, ok) = GFp5::decode(bytes);
        (ok != 0).then_some(Self(element))
    }

    pub fn to_bytes(self) -> [u8; 40] {
        self.0.encode()
    }
}

/// Hash an ASCII message into a digest.
///
/// Returns `None` when a chunk is not a canonical field element, which is what
/// the venue's `ArrayFromCanonicalLittleEndianBytes` does. The auth preimage is
/// digits and colons so it can never trip, but a digest that silently reduced
/// non-canonical input would be a second encoding of the same message.
pub fn digest(message: &str) -> Option<Digest> {
    let mut limbs = Vec::with_capacity(message.len().div_ceil(8));
    for chunk in message.as_bytes().chunks(8) {
        let mut eight = [0u8; 8];
        eight[..chunk.len()].copy_from_slice(chunk);
        let (limb, ok) = GFp::from_u64(u64::from_le_bytes(eight));
        if ok == 0 {
            return None;
        }
        limbs.push(limb);
    }
    Some(Digest(hash_to_quintic_extension(&limbs)))
}

/// A 40-byte ECgFp5 private key.
///
/// Deliberately not `Clone`, not `Copy` (the `Drop` below forbids it) and not
/// `Display`; `Debug` is redacted. A trading key that can be duplicated or
/// formatted is a key that ends up in a log line.
pub struct PrivateKey(Scalar);

impl PrivateKey {
    pub fn from_hex(text: &str) -> Result<Self, SignError> {
        let mut bytes = unhex(text).ok_or(SignError::PrivateKey)?;
        if bytes.len() != 40 {
            return Err(SignError::PrivateKey);
        }
        // The venue reduces rather than rejects here, so a key it accepts is a
        // key this accepts — with one exception, below.
        let scalar = Scalar::decode_reduce(&bytes);
        // The decode ran through the heap; that copy is wiped before the
        // allocation goes back to the allocator. See the note on `Drop`.
        bytes.fill(0);
        std::hint::black_box(&mut bytes);

        // Zero is not a key. Its public key is `0*G`, the neutral, and every
        // pair `(s, H(s*G || m))` closes the verification equation there — so
        // a zero key mints tokens that verify against nothing and identify
        // nobody. It is also the shape an uninitialised or half-written buffer
        // takes, which is how one gets here in practice. Every multiple of the
        // group order reduces to zero too, so the test is on the reduced
        // scalar rather than on the bytes.
        if scalar.iszero() != 0 {
            return Err(SignError::PrivateKey);
        }
        Ok(Self(scalar))
    }

    /// The public key: `sk*G` compressed to one Fp5 element.
    pub fn public_key(&self) -> [u8; 40] {
        Point::mulgen(self.0).encode().encode()
    }

    /// Sign a digest. `(s || e)`, 40 bytes each, little endian.
    ///
    /// The nonce is derived from the key and the digest instead of sampled.
    /// The venue's signer samples, and either is verifiable — but a sampled
    /// nonce makes the whole key depend on the quality of an RNG at signing
    /// time, and repeating one nonce across two digests publishes the key.
    /// Deriving it removes that failure mode and costs nothing here, since a
    /// token is signed rarely and never in a loop.
    pub fn sign(&self, message: Digest) -> [u8; 80] {
        let k = self.nonce(message);
        let r = Point::mulgen(k).encode();
        let e = challenge(r, message);
        let s = k - e * self.0;

        let mut out = [0u8; 80];
        out[..40].copy_from_slice(&s.encode());
        out[40..].copy_from_slice(&e.encode());
        out
    }

    /// `k = H(domain || sk || digest)` squeezed to 640 bits before reduction.
    ///
    /// Ten limbs rather than five, which is what the challenge uses: the order
    /// is 0.9999999988 of 2^319, so reducing a 320-bit hash into it leaves
    /// roughly 2^-30 of the range with one extra preimage. That bias is
    /// harmless in a public challenge and is exactly what lattice attacks eat
    /// in a nonce. Squeezing 640 bits first pushes it under 2^-320.
    ///
    /// The leading tag makes this preimage eleven elements where the
    /// challenge's is ten, so the two hashes cannot be handed the same input;
    /// the secret limbs in the middle are what make the result unguessable.
    fn nonce(&self, message: Digest) -> Scalar {
        let mut key = self.0.encode();
        let mut preimage = [GFp::ZERO; 11];
        preimage[0] = GFp::from_u64_reduce(NONCE_DOMAIN);
        for (i, chunk) in key.chunks(8).enumerate() {
            preimage[1 + i] = GFp::from_u64_reduce(u64::from_le_bytes(chunk.try_into().unwrap()));
        }
        preimage[6..].copy_from_slice(&message.0.0);

        let mut wide = [GFp::ZERO; 10];
        hash_n_to_m(&preimage, &mut wide);
        let mut bytes = [0u8; 80];
        for (i, limb) in wide.iter().enumerate() {
            bytes[i * 8..i * 8 + 8].copy_from_slice(&limb.to_u64().to_le_bytes());
        }
        let nonce = Scalar::decode_reduce(&bytes);

        // `key` and the first six limbs of `preimage` are the key; `wide` and
        // `bytes` are the nonce, which publishes the key if it is ever seen
        // beside a signature. All four are named locals, so all four are
        // wiped. See the note on `Drop` for what is not.
        key.fill(0);
        preimage.fill(GFp::ZERO);
        wide.fill(GFp::ZERO);
        bytes.fill(0);
        std::hint::black_box((&mut key, &mut preimage, &mut wide, &mut bytes));

        nonce
    }
}

/// Arbitrary and fixed. Changing it changes every signature this module
/// produces and none that it verifies, so it is not a compatibility surface.
const NONCE_DOMAIN: u64 = 0x4c_49_47_48_54_45_52_00; // "LIGHTER\0"

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateKey(<redacted>)")
    }
}

/// Overwrites the one copy of the key this type owns.
///
/// It is worth being exact about what that is and is not, because "the key is
/// zeroized" is a larger claim than anything here can deliver:
///
/// - **Wiped.** This scalar; the heap `Vec` `from_hex` decodes through (the
///   only copy that outlives a stack frame, and the one an allocator can hand
///   to the next caller); the byte and limb buffers in `nonce`.
/// - **Not wiped, and unreachable.** `Scalar` is `Copy`, so `self.0` is copied
///   by value into `Point::mulgen`, into `e * self.0`, and into every
///   intermediate the reference crate builds from those. Where those copies
///   live — registers, spill slots, a `memcpy` of the argument — is the
///   optimiser's choice, and safe Rust cannot name them, let alone write over
///   them. A `zeroize` dependency would not reach them either; it wipes what
///   it is handed, which is the same set as above.
///
/// So this destructor bounds the copies the module *names*, not the ones the
/// machine makes. Its other job is structural and does hold absolutely: while
/// it exists, `PrivateKey` cannot be `Copy`.
impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.0 = Scalar::ZERO;
        std::hint::black_box(&mut self.0);
    }
}

/// `e = H(r || H(m))`, reduced into the scalar field.
fn challenge(r: GFp5, message: Digest) -> Scalar {
    let mut preimage = [GFp::ZERO; 10];
    preimage[..5].copy_from_slice(&r.0);
    preimage[5..].copy_from_slice(&message.0.0);
    Scalar::decode_reduce(&hash_to_quintic_extension(&preimage).encode())
}

/// Verify `(s || e)` against a compressed public key.
///
/// `s*G + e*pk` is recomputed and rehashed; the signature is good when the
/// challenge falls out again. ECgFp5 has prime order, so a point that decodes
/// is a group element and there is no subgroup check to forget — the one
/// element that still has to be turned away is the neutral, which is a group
/// element and not a key.
pub fn verify(public_key: &[u8; 40], message: Digest, signature: &[u8; 80]) -> bool {
    let (s, s_ok) = Scalar::decode(&signature[..40]);
    let (e, e_ok) = Scalar::decode(&signature[40..]);
    // Non-canonical s or e is a second encoding of the same signature. The
    // venue reduces them instead, so it accepts a few forms this rejects;
    // rejecting is the direction that cannot admit a forgery.
    if (s_ok & e_ok) == 0 {
        return false;
    }
    let (encoded, key_ok) = GFp5::decode(public_key);
    if key_ok == 0 {
        return false;
    }
    // The neutral is `0*G`, and under it `s*G + e*pk` is just `s*G`: `e` loses
    // its only tie to the key, so any `(s, H(s*G || m))` verifies with nothing
    // signed. Its encoding is the all-zero field element, which is the shape
    // of an uninitialised buffer, so it is a likely thing to be handed.
    //
    // Refused here, on the encoding, rather than after the decode: a key that
    // is no point at all decodes to the neutral too, so a check on the decoded
    // point would answer for both and leave the decode check below untestable.
    if encoded.iszero() != 0 {
        return false;
    }
    let (point, point_ok) = Point::decode(encoded);
    if point_ok == 0 {
        return false;
    }
    challenge((Point::mulgen(s) + point * e).encode(), message).equals(e) != 0
}

// ---------------------------------------------------------------------------
// The auth token.

/// The venue rejects a deadline more than this far ahead as `invalid
/// deadline`; established by bisecting live refusals, the bound is exact.
const MAX_LIFETIME: u64 = 8 * 60 * 60;

#[derive(Debug, PartialEq, Eq)]
pub enum SignError {
    /// Never carries the text it rejected: a mistyped key is still a key.
    PrivateKey,
    Message,
    DeadlinePassed,
    DeadlineTooFar,
    /// A transaction field a digest cannot carry, named. See [`field`].
    Field(&'static str),
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrivateKey => {
                f.write_str("private key must be 40 bytes of hex, and not zero mod the order")
            }
            Self::Message => f.write_str("auth message is not a canonical field element"),
            Self::DeadlinePassed => f.write_str("deadline is not in the future"),
            Self::DeadlineTooFar => f.write_str("deadline is more than 8h ahead"),
            Self::Field(name) => write!(f, "a transaction's {name} cannot be negative"),
        }
    }
}

impl std::error::Error for SignError {}

/// Build `{deadline}:{account}:{api_key}:{signature}` for `deadline` given as
/// a Unix timestamp in seconds.
///
/// The deadline is checked here rather than left to the venue because the
/// token is not bound to a route: every gated read accepts it, so its lifetime
/// is the only thing limiting what a copy of it can do. A token minted outside
/// `(now, now + 8h]` is refused by the venue anyway, so accepting one would
/// only mean discovering that over the wire.
pub fn auth_token(
    key: &PrivateKey,
    account_index: i64,
    api_key_index: u8,
    deadline: u64,
) -> Result<String, SignError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    if deadline <= now {
        return Err(SignError::DeadlinePassed);
    }
    if deadline - now > MAX_LIFETIME {
        return Err(SignError::DeadlineTooFar);
    }

    let message = format!("{deadline}:{account_index}:{api_key_index}");
    let signature = key.sign(digest(&message).ok_or(SignError::Message)?);
    Ok(format!("{message}:{}", hex(&signature)))
}

fn hex(bytes: &[u8]) -> String {
    use fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    let text = text.strip_prefix("0x").unwrap_or(text);
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

// ---------------------------------------------------------------------------
// The L2 transactions.

/// The numbers the sequencer files these two under. They are the `tx_type` the
/// submission carries *and* the second element of the digest, so a transaction
/// cannot be re-filed as another kind after it is signed.
const TX_CREATE_ORDER: u8 = 14;
const TX_CANCEL_ORDER: u8 = 15;

/// The one order *type* this app places, as the venue's own tables number it: a
/// limit order, priced by the ticket and never by a trigger.
///
/// The type is fixed where the resting rule is not. A stop or a take-profit is
/// a different type with its own validation table and its own trigger price,
/// and this app attaches neither on this venue — `venue_attaches_levels` says
/// so on the ticket. How long the order rests, though, is a control the reader
/// has, so that one is carried.
const ORDER_LIMIT: i64 = 0;
const NO_TRIGGER: i64 = 0;

/// How long a Lighter order rests, in the venue's own numbering.
///
/// The same three the ticket offers, and the mapping is not quite the naming:
/// this venue has no rest-until-cancelled, so its longest-lived order carries
/// the deadline it was signed with — which is what `venue_tif_note` tells the
/// reader and why `Resting::Deadline` is not called `Gtc` here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resting {
    /// Take what is on the book now and cancel the rest. Carries no expiry,
    /// and the venue refuses one that does.
    Immediate,
    /// Rest until the expiry the order carries. Requires that expiry.
    Deadline,
    /// Never cross: a maker order or nothing.
    PostOnly,
}

impl Resting {
    fn code(self) -> i64 {
        match self {
            Resting::Immediate => 0,
            Resting::Deadline => 1,
            Resting::PostOnly => 2,
        }
    }

    /// Whether an order resting this way carries an expiry at all. The venue
    /// validates the pairing rather than ignoring it: an immediate order with
    /// an expiry is refused, and a resting one without is refused too.
    pub fn expires(self) -> bool {
        !matches!(self, Resting::Immediate)
    }
}

/// One limit order, as the transaction carries it.
///
/// Integers throughout, because that is what is signed: a price is counted in
/// the market's own price steps and a size in its size steps, and the adapter
/// that reads a market's decimals is what turns a figure on screen into one of
/// these. A float here would be a rounding rule living in the signer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewOrder {
    pub account: i64,
    pub api_key: u8,
    /// The market's own id on *this* deployment. The two deployments number
    /// their markets independently, so an id is meaningless without the zone
    /// beside it — which is why both reach the digest together.
    pub market: i16,
    /// The app's own name for this order, and the only handle it gets: the
    /// venue answers a submission with a transaction hash rather than an order
    /// id, so the cancel names the order by the index its placer chose.
    pub client_index: i64,
    pub base_amount: i64,
    pub price: u32,
    pub ask: bool,
    pub reduce_only: bool,
    pub resting: Resting,
    /// When the resting order stops resting, and zero for one that never rests.
    /// `Resting::expires` is which of those this must be.
    pub expiry_ms: i64,
    /// When the *transaction* stops being submittable, which is a different
    /// deadline and a far shorter one: it bounds how long a signed transaction
    /// somebody copied is worth replaying, and says nothing about the order.
    pub deadline_ms: i64,
    pub nonce: i64,
}

/// One resting order to pull, named by the index it was placed under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancel {
    pub account: i64,
    pub api_key: u8,
    pub market: i16,
    pub index: i64,
    pub deadline_ms: i64,
    pub nonce: i64,
}

/// A built, unsigned transaction: the number the wire files it under, the
/// digest the API key signs, and the body those same fields go out as.
///
/// The digest is built here rather than by the caller for the reason
/// `signing.rs` builds Hyperliquid's the same way: what is signed and what is
/// sent have to come from one set of values, or a field can be signed in one
/// shape and sent in another and the venue recovers a stranger.
#[derive(Clone, Debug)]
pub struct Transaction {
    tx_type: u8,
    digest: Digest,
    fields: String,
}

impl Transaction {
    pub fn tx_type(&self) -> u8 {
        self.tx_type
    }

    pub fn digest(&self) -> Digest {
        self.digest
    }

    /// The `tx_info` the submission carries, with this key's signature in it.
    ///
    /// `L2TxAttributes` is always null: attributes are integrator fees and
    /// self-trade modes, this app sets none, and an empty set is the one case
    /// the venue's digest leaves out of the hash entirely.
    pub fn signed(&self, key: &PrivateKey) -> String {
        let signature = BASE64.encode(key.sign(self.digest));
        format!(
            "{{{},\"Sig\":\"{signature}\",\"L2TxAttributes\":null}}",
            self.fields
        )
    }
}

/// Build the digest for a create-order transaction.
///
/// The element order is the venue's and is not the body's: the chain and the
/// transaction type lead, then the two the sequencer replays against, then the
/// transaction's own fields. Transcribed from `lighter-go`'s
/// `L2CreateOrderTxInfo.Hash`, and pinned against digests that signer produced.
pub fn create_order(zone: Zone, order: &NewOrder) -> Result<Transaction, SignError> {
    let ask = i64::from(order.ask);
    let reduce_only = i64::from(order.reduce_only);
    let digest = digest_of(&[
        ("chain id", i64::from(zone.chain_id())),
        ("transaction type", i64::from(TX_CREATE_ORDER)),
        ("nonce", order.nonce),
        ("deadline", order.deadline_ms),
        ("account index", order.account),
        ("api key index", i64::from(order.api_key)),
        ("market index", i64::from(order.market)),
        ("client order index", order.client_index),
        ("base amount", order.base_amount),
        ("price", i64::from(order.price)),
        ("side", ask),
        ("order type", ORDER_LIMIT),
        ("time in force", order.resting.code()),
        ("reduce-only flag", reduce_only),
        ("trigger price", NO_TRIGGER),
        ("order expiry", order.expiry_ms),
    ])?;
    Ok(Transaction {
        tx_type: TX_CREATE_ORDER,
        digest,
        fields: format!(
            "\"AccountIndex\":{},\"ApiKeyIndex\":{},\"MarketIndex\":{},\
             \"ClientOrderIndex\":{},\"BaseAmount\":{},\"Price\":{},\"IsAsk\":{ask},\
             \"Type\":{ORDER_LIMIT},\"TimeInForce\":{},\
             \"ReduceOnly\":{reduce_only},\"TriggerPrice\":{NO_TRIGGER},\
             \"OrderExpiry\":{},\"ExpiredAt\":{},\"Nonce\":{}",
            order.account,
            order.api_key,
            order.market,
            order.client_index,
            order.base_amount,
            order.price,
            order.resting.code(),
            order.expiry_ms,
            order.deadline_ms,
            order.nonce,
        ),
    })
}

/// The same for a cancel, whose digest is the same envelope over half the
/// fields.
pub fn cancel_order(zone: Zone, cancel: &Cancel) -> Result<Transaction, SignError> {
    let digest = digest_of(&[
        ("chain id", i64::from(zone.chain_id())),
        ("transaction type", i64::from(TX_CANCEL_ORDER)),
        ("nonce", cancel.nonce),
        ("deadline", cancel.deadline_ms),
        ("account index", cancel.account),
        ("api key index", i64::from(cancel.api_key)),
        ("market index", i64::from(cancel.market)),
        ("order index", cancel.index),
    ])?;
    Ok(Transaction {
        tx_type: TX_CANCEL_ORDER,
        digest,
        fields: format!(
            "\"AccountIndex\":{},\"ApiKeyIndex\":{},\"MarketIndex\":{},\"Index\":{},\
             \"ExpiredAt\":{},\"Nonce\":{}",
            cancel.account,
            cancel.api_key,
            cancel.market,
            cancel.index,
            cancel.deadline_ms,
            cancel.nonce,
        ),
    })
}

/// Hash the named fields, refusing any that a digest cannot carry.
///
/// The venue casts each field straight into a Goldilocks element, so a
/// negative one wraps to an element near 2^64 rather than to a small one.
/// Nothing here is ever legitimately negative, and a value that arrived that
/// way would be signed as a number nobody meant — so it is refused by name
/// rather than hashed. Everything non-negative needs no further check: the
/// Goldilocks prime is 2^64 - 2^32 + 1, which is above `i64::MAX`, so every
/// value that passes this is already canonical.
fn digest_of(fields: &[(&'static str, i64)]) -> Result<Digest, SignError> {
    let mut elements = Vec::with_capacity(fields.len());
    for (name, value) in fields {
        if *value < 0 {
            return Err(SignError::Field(name));
        }
        elements.push(GFp::from_u64_reduce(*value as u64));
    }
    Ok(Digest(hash_to_quintic_extension(&elements)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway key pair the venue's own signer produced
    /// (`lighter-python`'s `lighter-signer-linux-amd64.so`, sha256
    /// e18ff6bad3b49c4fc17c2a9bbf0fba8430f6be84bcc4abbe35a2e2f4641963f9, a cgo
    /// build of the open `lighter-go`). It is registered nowhere, which is what
    /// makes it safe to sign with and useful for `reaches_the_signature_check`.
    const PRIVATE_KEY: &str =
        "0x64ca3ac2840332193cf362603055e0808e039bc143e965f0b0aa922a1a4d40d5af86c4cb0cd07370";
    const PUBLIC_KEY: &str =
        "cb92c72468df173cab282606e6a8ee8ef94e965def5215f9688c0134cda1c401babe8a04576e64df";

    const ACCOUNT: i64 = 702384;
    const API_KEY: u8 = 3;
    const MESSAGE: &str = "1786201085:702384:3";
    const DIGEST: &str =
        "f455b65c8b73e6cb068a2d1ddd437893415b94f6a52e7ab6dce78110e2390f6881d260b57910b458";

    /// A token that same signer emitted over `MESSAGE` with `PRIVATE_KEY`. Its
    /// nonce was sampled, so this exact signature is one this module would
    /// never produce — which is the point of verifying it.
    const OFFICIAL_SIGNATURE: &str = concat!(
        "fa6313d9ec4182168a8d632c7b63a58e5a99fdb51139c0a6f0e40387a9b1196d",
        "64bbaac70c04bd2021ddd6e64335bd52552d58f9958a6873725578f9418bafb1",
        "43a691773936b4ade780c2d4dd941c7d",
    );

    fn key() -> PrivateKey {
        PrivateKey::from_hex(PRIVATE_KEY).expect("the oracle key is 40 bytes of hex")
    }

    fn bytes<const N: usize>(text: &str) -> [u8; N] {
        unhex(text).expect("test vector is hex")[..]
            .try_into()
            .expect("test vector length")
    }

    fn official_signature() -> [u8; 80] {
        bytes(OFFICIAL_SIGNATURE)
    }

    /// The group order n, little-endian limbs.
    ///
    /// The curve crate keeps its own copy private on purpose ("this constant
    /// MUST NOT leak outside the API"), so this is a transcription, and every
    /// use below asserts what it means rather than trusting it: `0 + n`
    /// reduces to zero, and `s + n` reduces to `s`.
    const ORDER: [u64; 5] = [
        0xE80F_D996_948B_FFE1,
        0xE888_5C39_D724_A09C,
        0x7FFF_FFE6_CFB8_0639,
        0x7FFF_FFF1_0000_0016,
        0x7FFF_FFFD_8000_0007,
    ];

    /// `value + n` in the same 40 bytes: a second encoding of the same scalar.
    /// Every scalar is below n and n is below 2^319, so the sum never leaves
    /// the encoding — asserted, not assumed.
    fn plus_order(value: &[u8]) -> [u8; 40] {
        let mut out = [0u8; 40];
        let mut carry = 0u64;
        for (i, order) in ORDER.iter().enumerate() {
            let limb = u64::from_le_bytes(value[i * 8..i * 8 + 8].try_into().expect("eight bytes"));
            let (sum, over) = limb.overflowing_add(*order);
            let (sum, wrapped) = sum.overflowing_add(carry);
            carry = u64::from(over | wrapped);
            out[i * 8..i * 8 + 8].copy_from_slice(&sum.to_le_bytes());
        }
        assert_eq!(carry, 0, "value + n does not fit 40 bytes");
        out
    }

    /// A signature that closes the verification equation at the neutral point
    /// without signing anything: there `s*G + e*pk` is just `s*G`, so
    /// `(s, H(s*G || m))` verifies for any `s` at all — here `s = 1`.
    ///
    /// It is the forgery a public key gets to accept if it is the neutral, or
    /// if a key that fails to decode falls through to the neutral. Offering it
    /// is what makes those two tests fail when the refusals are removed;
    /// against a real key it is refused for the ordinary reason.
    fn neutral_forgery(message: Digest) -> [u8; 80] {
        let s = Scalar::ONE;
        let mut out = [0u8; 80];
        out[..40].copy_from_slice(&s.encode());
        out[40..].copy_from_slice(&challenge(Point::mulgen(s).encode(), message).encode());
        out
    }

    /// The whole hash stack in one number. `HashToQuinticExtension` over the
    /// ASCII preimage, chunked eight bytes at a time into Goldilocks limbs, has
    /// to land on the digest the venue's signer computed for the same string —
    /// so this pins the Poseidon2 constants, the fused round order, the
    /// unpadded sponge and the limb packing together.
    #[test]
    fn the_digest_matches_the_official_signer() {
        let computed = digest(MESSAGE).expect("ASCII is canonical");
        assert_eq!(hex(&computed.to_bytes()), DIGEST);
    }

    /// Key derivation against the pair the official signer emitted together.
    /// Nothing here is self-referential: the private key went in, the public
    /// key came out of their code, and `sk*G` compressed has to reproduce it.
    #[test]
    fn the_public_key_derives_from_the_private_key() {
        assert_eq!(hex(&key().public_key()), PUBLIC_KEY);
    }

    /// The proof that matters. This signature was produced by the venue's
    /// signer with a nonce this module cannot reproduce, so accepting it means
    /// the challenge hash, the scalar reduction, the point decompression and
    /// `s*G + e*pk` all agree with theirs — not merely with each other.
    #[test]
    fn a_signature_from_the_official_signer_verifies() {
        assert!(verify(
            &bytes(PUBLIC_KEY),
            digest(MESSAGE).expect("ASCII is canonical"),
            &official_signature(),
        ));
    }

    /// The other half: a verifier that accepts everything proves nothing. The
    /// same official signature must fail against a digest one bit away.
    #[test]
    fn the_official_signature_fails_against_a_tampered_digest() {
        let mut tampered = digest(MESSAGE).expect("ASCII is canonical").to_bytes();
        tampered[0] ^= 1;
        assert!(!verify(
            &bytes(PUBLIC_KEY),
            Digest::from_bytes(&tampered).expect("flipping a low bit stays canonical"),
            &official_signature(),
        ));
        // And against a message that differs only in its last deadline digit.
        assert!(!verify(
            &bytes(PUBLIC_KEY),
            digest("1786201086:702384:3").expect("ASCII is canonical"),
            &official_signature(),
        ));
    }

    /// Signing is the direction the oracle cannot pin, since this module
    /// derives its nonce where the venue samples one. What it can pin is that
    /// the signature lands in the same group: their verifier's equation,
    /// re-run here, has to accept it.
    #[test]
    fn a_signature_this_module_produced_verifies_under_the_same_equation() {
        let message = digest(MESSAGE).expect("ASCII is canonical");
        let signature = key().sign(message);
        assert_ne!(
            signature,
            official_signature(),
            "a derived nonce is not their sampled one"
        );
        assert!(verify(&bytes(PUBLIC_KEY), message, &signature));
    }

    /// A signature must not carry over to another key, which is the check that
    /// fails if `e` were folded in without the public key ever entering the
    /// verification equation.
    #[test]
    fn a_signature_does_not_verify_under_another_public_key() {
        let other = PrivateKey::from_hex(&"11".repeat(40)).expect("40 bytes of hex");
        let message = digest(MESSAGE).expect("ASCII is canonical");
        assert!(!verify(&other.public_key(), message, &official_signature()));
    }

    /// Malleability, and the module's one deliberate divergence from the
    /// venue: `s` and `e` are integers mod n, so `s + n` is a second encoding
    /// of the same signature, which the venue reduces and accepts and this
    /// refuses.
    ///
    /// So the input has to be that second encoding and not merely a corrupt
    /// one — garbage in `s` is refused by the signature equation whether
    /// anything checks canonicity or not, and proves nothing about the check.
    /// The two premises are asserted here: `s + n` is not canonical, and it
    /// reduces back to `s`. A verifier that reduced first would therefore
    /// accept it, and only one that checks canonicity refuses.
    #[test]
    fn a_non_canonical_scalar_is_refused() {
        let message = digest(MESSAGE).expect("ASCII is canonical");
        for half in [0, 40] {
            let mut signature = official_signature();
            let shifted = plus_order(&signature[half..half + 40]);
            assert_eq!(
                Scalar::decode(&shifted).1,
                0,
                "premise: the shifted scalar is not canonical"
            );
            assert_eq!(
                &Scalar::decode_reduce(&shifted).encode()[..],
                &signature[half..half + 40],
                "premise: the shifted scalar reduces to the original"
            );

            signature[half..half + 40].copy_from_slice(&shifted);
            assert!(
                !verify(&bytes(PUBLIC_KEY), message, &signature),
                "a second encoding of the signature was accepted at byte {half}"
            );
        }
    }

    /// A public key that is not on the curve has to be refused rather than
    /// decoded to the neutral point and hashed anyway.
    ///
    /// Flipping a bit of the real key is not this test: half of all field
    /// elements decode, and that one does, so it only ever exercised the
    /// ordinary mismatch. The key here is the field element 1, which is
    /// canonical (asserted) and is the encoding of no point (asserted), and
    /// the signature offered with it is the one that satisfies the equation at
    /// the neutral — so falling through to the neutral accepts a forgery.
    #[test]
    fn a_public_key_that_does_not_decode_is_refused() {
        let mut public_key = [0u8; 40];
        public_key[0] = 1;
        let (element, canonical) = GFp5::decode(&public_key);
        assert_ne!(canonical, 0, "premise: the limbs are canonical");
        assert_eq!(Point::validate(element), 0, "premise: w = 1 is not a point");

        let message = digest(MESSAGE).expect("ASCII is canonical");
        assert!(!verify(&public_key, message, &neutral_forgery(message)));
    }

    /// The neutral is the other half of that: it *does* decode, so the decode
    /// check alone lets it past, and it is `0*G` — the public key of the zero
    /// private key, and the all-zero buffer besides. Under it the verification
    /// equation loses the key entirely and accepts a signature over nothing.
    #[test]
    fn the_neutral_public_key_is_refused() {
        let (element, canonical) = GFp5::decode(&[0u8; 40]);
        assert_ne!(canonical, 0, "premise: the limbs are canonical");
        assert_ne!(
            Point::validate(element),
            0,
            "premise: the neutral decodes, which is why it needs its own check"
        );

        let message = digest(MESSAGE).expect("ASCII is canonical");
        assert!(!verify(&[0u8; 40], message, &neutral_forgery(message)));
    }

    /// A private key that reduces to zero is refused at the door.
    ///
    /// Zero is the shape of an uninitialised or half-written buffer, and it is
    /// the one scalar with no key in it: `0*G` is the neutral, so the token it
    /// signs verifies against a public key that identifies nobody and accepts
    /// anybody. `from_hex` reduces its input, so the check has to be on the
    /// reduced scalar — n itself is 40 bytes of ordinary-looking hex.
    #[test]
    fn a_private_key_that_reduces_to_zero_is_refused() {
        let order = plus_order(&[0u8; 40]);
        assert_ne!(
            Scalar::decode_reduce(&order).iszero(),
            0,
            "premise: n reduces to zero, so the transcribed order is right"
        );

        for text in ["00".repeat(40), hex(&order)] {
            assert_eq!(
                PrivateKey::from_hex(&text).unwrap_err(),
                SignError::PrivateKey,
                "accepted a private key that is zero mod the order"
            );
        }
        assert!(
            PrivateKey::from_hex(PRIVATE_KEY).is_ok(),
            "a real key still"
        );
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after 1970")
            .as_secs()
    }

    /// The shape the venue parses, and the only three fields under the
    /// signature.
    #[test]
    fn the_token_is_the_message_followed_by_its_signature() {
        let deadline = now() + 600;
        let token = auth_token(&key(), ACCOUNT, API_KEY, deadline).expect("inside the window");

        let (message, signature) = token.rsplit_once(':').expect("four colon-separated fields");
        assert_eq!(message, format!("{deadline}:{ACCOUNT}:{API_KEY}"));
        assert_eq!(signature.len(), 160, "80 bytes, hex");
        assert!(verify(
            &key().public_key(),
            digest(message).expect("ASCII is canonical"),
            &bytes(signature),
        ));
    }

    /// The token is not bound to a route, so its deadline is the only thing
    /// limiting a copy of it. Both edges of the window the venue enforces are
    /// refused here rather than over the wire.
    ///
    /// The seconds are spelled out rather than written in terms of
    /// `MAX_LIFETIME`: 28800 is the venue's bound, found by bisecting live
    /// refusals, and a test phrased against the constant would agree with any
    /// value the constant took. The five seconds of slack absorb the tick
    /// between this clock reading and `auth_token`'s.
    #[test]
    fn a_deadline_outside_the_window_is_refused() {
        let key = key();
        assert_eq!(
            auth_token(&key, ACCOUNT, API_KEY, now()),
            Err(SignError::DeadlinePassed)
        );
        assert_eq!(
            auth_token(&key, ACCOUNT, API_KEY, now() - 1),
            Err(SignError::DeadlinePassed)
        );
        assert_eq!(
            auth_token(&key, ACCOUNT, API_KEY, now() + 28_805),
            Err(SignError::DeadlineTooFar)
        );
        assert!(auth_token(&key, ACCOUNT, API_KEY, now() + 28_795).is_ok());
    }

    /// Nothing that can be printed may print the key, and nothing that rejects
    /// a key may quote it back. `Debug` is the accident waiting to happen: a
    /// derive here would put five secret limbs into any log line that formats
    /// a struct holding one.
    ///
    /// The guarantee is stated as *`Debug` tells two keys apart in no way*,
    /// because a scan for the key's own spelling does not state it: a derive
    /// prints the limbs in decimal, and a scan for the hex the key was typed
    /// in walks straight past that. Pinning the redaction's exact text does
    /// not state it either — that catches an edit to the string and calls it a
    /// leak. Formatting two different keys and comparing is spelling-proof and
    /// says the thing: whatever `Debug` prints, none of it came from the key.
    ///
    /// The substring scan stays for the paths that take the key as *text* —
    /// `from_hex`'s error, which must not echo what it rejected — and it scans
    /// both spellings, hex as typed and limbs in decimal.
    #[test]
    fn no_printable_path_carries_key_material() {
        let key = key();
        let other = PrivateKey::from_hex(&"11".repeat(40)).expect("40 bytes of hex");
        assert_eq!(
            format!("{key:?}"),
            format!("{other:?}"),
            "Debug distinguishes two keys, so it is carrying some of them"
        );

        let stripped = PRIVATE_KEY.trim_start_matches("0x");
        let mut secrets: Vec<String> = Vec::new();
        for spelling in [stripped, &stripped.to_uppercase()] {
            secrets.extend((0..spelling.len() - 8).map(|start| spelling[start..start + 8].into()));
        }
        secrets.extend(
            bytes::<40>(PRIVATE_KEY)
                .chunks(8)
                .map(|limb| u64::from_le_bytes(limb.try_into().expect("eight bytes")).to_string()),
        );

        for text in [
            format!("{key:?}"),
            SignError::PrivateKey.to_string(),
            format!("{:?}", SignError::PrivateKey),
            format!("{:?}", PrivateKey::from_hex(stripped).map(|_| ())),
            format!("{:?}", PrivateKey::from_hex("not hex").unwrap_err()),
        ] {
            for secret in &secrets {
                assert!(!text.contains(secret), "leaked {secret}: {text}");
            }
        }
    }

    /// What the destructor is checkable for, which is less than its name used
    /// to claim.
    ///
    /// Checked here: `PrivateKey` has drop glue. That is the structural half,
    /// and it is exact — a type with a destructor cannot be `Copy`, and with
    /// no `Clone` either that leaves one live copy of the scalar per key,
    /// enforced by the compiler at every call site.
    ///
    /// Not checked, by anything, anywhere: that the destructor's overwrite
    /// reaches memory. Once the value drops it is unnameable, and reading the
    /// freed frame back needs `unsafe`, which this workspace forbids — so
    /// gutting the body of `Drop` fails no test in this file. The note on
    /// `Drop` says which copies that overwrite is aimed at and which ones no
    /// safe implementation could reach.
    #[test]
    fn the_private_key_has_a_destructor_so_it_cannot_be_copy() {
        assert!(
            std::mem::needs_drop::<PrivateKey>(),
            "PrivateKey lost its Drop, so it can be made Copy and its limbs outlive it"
        );
    }

    // -----------------------------------------------------------------------
    // The transactions, against the same signer the token's vectors came from.
    //
    // Each vector is one call into `lighter-signer-linux-amd64.so` with the
    // inputs spelled below, and what came back is the digest and the body. The
    // `ExpiredAt` in each is not a choice: the shared library stamps it from
    // its own clock, so it is recorded here exactly as it was produced. That
    // makes these fixed vectors rather than a re-derivation, which is the
    // point — they were produced by their code, not by ours.
    // -----------------------------------------------------------------------

    /// A limit buy on the test deployment, at the small end of every range.
    const BUY: NewOrder = NewOrder {
        account: ACCOUNT,
        api_key: API_KEY,
        market: 1,
        client_index: 13,
        base_amount: 100_000,
        price: 6_500_000,
        ask: false,
        reduce_only: false,
        resting: Resting::Deadline,
        expiry_ms: 1_786_300_000_000,
        deadline_ms: 1_786_279_157_060,
        nonce: 7,
    };
    const BUY_DIGEST: &str =
        "5be034139e72ab860d2f11051a10d878b991b69a12b3cec3d360a7cbecda31e5f9492d0f6970bd30";
    const BUY_BODY: &str = concat!(
        r#"{"AccountIndex":702384,"ApiKeyIndex":3,"MarketIndex":1,"ClientOrderIndex":13,"#,
        r#""BaseAmount":100000,"Price":6500000,"IsAsk":0,"Type":0,"TimeInForce":1,"#,
        r#""ReduceOnly":0,"TriggerPrice":0,"OrderExpiry":1786300000000,"#,
        r#""ExpiredAt":1786279157060,"Nonce":7,"Sig":"botQoZ4SpebwpsSbz1vOQGhjQwRIlJJU1einQ"#,
        r#"DhXggAf4IvvwCbOYakGncaoc4TmOuuGPRNu20s0qD9777jdbL/ZHxaOFLsvzwRhl5K4jWc=","#,
        r#""L2TxAttributes":null}"#,
    );

    /// The same order's cancel.
    const PULL: Cancel = Cancel {
        account: ACCOUNT,
        api_key: API_KEY,
        market: 1,
        index: 13,
        deadline_ms: 1_786_279_157_060,
        nonce: 9,
    };
    const PULL_DIGEST: &str =
        "f7b4080dd16cc13c626a3b43859cb34d4d1c3992bf1003eab418db07b1cdc97a7c706adb54c4333b";
    const PULL_BODY: &str = concat!(
        r#"{"AccountIndex":702384,"ApiKeyIndex":3,"MarketIndex":1,"Index":13,"#,
        r#""ExpiredAt":1786279157060,"Nonce":9,"Sig":"/HO8yYTU2Wqyrf6atHXJcRxpsjpIU9NuppSD"#,
        r#"fIimDFaX3X8nrm3oCY9qlYWWHtPQGb8f5/P/+5q2H0fbrMG3Q8NNnmvOXG/44pFgVkw3YlY=","#,
        r#""L2TxAttributes":null}"#,
    );

    /// A reduce-only sell on the live deployment, with every field the venue
    /// caps pushed to its cap: the largest market index, client index, size,
    /// price and nonce it will take. The pair with `BUY` is what says a field
    /// is carried rather than a constant — every one of them differs.
    const CAPPED_SELL: NewOrder = NewOrder {
        account: ACCOUNT,
        api_key: API_KEY,
        market: 254,
        client_index: 281_474_976_710_655,
        base_amount: 281_474_976_710_655,
        price: u32::MAX,
        ask: true,
        reduce_only: true,
        resting: Resting::Deadline,
        expiry_ms: 4_102_444_800_000,
        deadline_ms: 1_786_279_157_061,
        nonce: 281_474_976_710_655,
    };
    const CAPPED_SELL_DIGEST: &str =
        "4f033e25df0aeb784a445e7104a7703cbbc3ef00e9e79dda5a8e868e6ae55b01710de9c48be0d7f9";
    const CAPPED_SELL_BODY: &str = concat!(
        r#"{"AccountIndex":702384,"ApiKeyIndex":3,"MarketIndex":254,"#,
        r#""ClientOrderIndex":281474976710655,"BaseAmount":281474976710655,"#,
        r#""Price":4294967295,"IsAsk":1,"Type":0,"TimeInForce":1,"ReduceOnly":1,"#,
        r#""TriggerPrice":0,"OrderExpiry":4102444800000,"ExpiredAt":1786279157061,"#,
        r#""Nonce":281474976710655,"Sig":"y0KvygQ14ir/gkwPEPolCp3qhgdgfzAojN0OPAYElFAAq3R1"#,
        r#"Ie3cHQoks8opXNrTJNuTm9R9aX4auk/5OB/v0t3qTyUWckrILlM5KboHa3w=","L2TxAttributes":null}"#,
    );

    /// A cancel naming an order by the venue's own index rather than a client
    /// one, at the top of that range.
    const CAPPED_PULL: Cancel = Cancel {
        account: ACCOUNT,
        api_key: API_KEY,
        market: 254,
        index: 1_152_921_504_606_846_975,
        deadline_ms: 1_786_279_157_061,
        nonce: 1,
    };
    const CAPPED_PULL_DIGEST: &str =
        "efb1df807b4d6d49e9efd79970e431d377c434be6167a98bcf870814ae175c515a25c3527a30a592";
    const CAPPED_PULL_BODY: &str = concat!(
        r#"{"AccountIndex":702384,"ApiKeyIndex":3,"MarketIndex":254,"#,
        r#""Index":1152921504606846975,"ExpiredAt":1786279157061,"Nonce":1,"#,
        r#""Sig":"7koGy+5Ipy637VwBcbbV8PBTS8IDeQRK39oeTfi5D4ACCimFYjnSOG01lyk5UCBQTuUH9w9ukGCX"#,
        r#"IqbqSTCO/KhLnmBpeXLo3lrs9zI0Rgo=","L2TxAttributes":null}"#,
    );

    /// The signature out of a body the official signer produced, as bytes.
    fn signature_in(body: &str) -> [u8; 80] {
        let (_, rest) = body.split_once("\"Sig\":\"").expect("a signed body");
        let (encoded, _) = rest.split_once('"').expect("a closed string");
        BASE64
            .decode(encoded)
            .expect("the venue spells a signature in base64")[..]
            .try_into()
            .expect("80 bytes of signature")
    }

    /// A body with its signature blanked, so two bodies can be compared on
    /// everything the signature is not. They cannot be compared whole: their
    /// signer samples its nonce where this module derives one, so the same
    /// transaction is signed to different bytes by each and only the fields
    /// under the signature are the same.
    fn without_signature(body: &str) -> String {
        let (before, rest) = body.split_once("\"Sig\":\"").expect("a signed body");
        let (_, after) = rest.split_once('"').expect("a closed string");
        format!("{before}\"Sig\":\"…\"{after}")
    }

    /// The whole transaction layout in one number, four times over.
    ///
    /// This is the claim the rest of the order path stands on: which fields are
    /// hashed, in which order, under which chain id, and how the leading two
    /// elements are spelled. Nothing about it can be checked against itself —
    /// a digest agrees with whatever built it — so each of these is a digest
    /// the venue's own signer produced for inputs stated beside it.
    ///
    /// Both transaction kinds and both deployments, because the chain id and
    /// the transaction type are elements of the hash rather than routing: a
    /// module that ignored either would still produce a stable digest and would
    /// sign a mainnet order with a testnet signature.
    #[test]
    fn a_transaction_digest_matches_the_official_signer() {
        let digest = |transaction: Transaction| hex(&transaction.digest().to_bytes());
        assert_eq!(
            digest(create_order(Zone::Testnet, &BUY).expect("a buildable order")),
            BUY_DIGEST,
        );
        assert_eq!(
            digest(cancel_order(Zone::Testnet, &PULL).expect("a buildable cancel")),
            PULL_DIGEST,
        );
        assert_eq!(
            digest(create_order(Zone::Mainnet, &CAPPED_SELL).expect("a buildable order")),
            CAPPED_SELL_DIGEST,
        );
        assert_eq!(
            digest(cancel_order(Zone::Mainnet, &CAPPED_PULL).expect("a buildable cancel")),
            CAPPED_PULL_DIGEST,
        );
    }

    /// The body beside the signature, spelled the way the sequencer's own
    /// client spells it: the same field names, the same order, integers rather
    /// than strings, and a null attribute map.
    ///
    /// It is compared against the official signer's own output rather than
    /// against a hand-written expectation, so a field renamed or dropped on
    /// their side shows up here as a difference rather than as a rejection
    /// nobody can read.
    #[test]
    fn a_transaction_body_matches_the_official_signer() {
        let key = key();
        let body = |transaction: Transaction| transaction.signed(&key);
        for (built, official) in [
            (
                body(create_order(Zone::Testnet, &BUY).expect("an order")),
                BUY_BODY,
            ),
            (
                body(cancel_order(Zone::Testnet, &PULL).expect("a cancel")),
                PULL_BODY,
            ),
            (
                body(create_order(Zone::Mainnet, &CAPPED_SELL).expect("an order")),
                CAPPED_SELL_BODY,
            ),
            (
                body(cancel_order(Zone::Mainnet, &CAPPED_PULL).expect("a cancel")),
                CAPPED_PULL_BODY,
            ),
        ] {
            assert_eq!(without_signature(&built), without_signature(official));
            // And the one field that legitimately differs is still a signature
            // of the right size, rather than the field having gone missing.
            assert_eq!(signature_in(&built).len(), 80);
            assert_ne!(
                signature_in(&built),
                signature_in(official),
                "a derived nonce is not their sampled one"
            );
        }
    }

    /// The other direction, and the one a body comparison cannot give: the
    /// signature the venue's signer made over its digest has to verify against
    /// the digest this module computes.
    ///
    /// That is a stronger statement than the hex match above. It says the two
    /// digests are the same *element* under their signature scheme rather than
    /// the same forty bytes under ours, so a transposed limb or a different
    /// byte order would fail here even if it happened to spell alike.
    #[test]
    fn the_official_signature_over_a_transaction_verifies() {
        let public = bytes::<40>(PUBLIC_KEY);
        for (transaction, body) in [
            (
                create_order(Zone::Testnet, &BUY).expect("an order"),
                BUY_BODY,
            ),
            (
                cancel_order(Zone::Testnet, &PULL).expect("a cancel"),
                PULL_BODY,
            ),
            (
                create_order(Zone::Mainnet, &CAPPED_SELL).expect("an order"),
                CAPPED_SELL_BODY,
            ),
            (
                cancel_order(Zone::Mainnet, &CAPPED_PULL).expect("a cancel"),
                CAPPED_PULL_BODY,
            ),
        ] {
            assert!(
                verify(&public, transaction.digest(), &signature_in(body)),
                "the official signature does not verify against this digest",
            );
            // And this module's own signature over the same digest verifies
            // too, which is what the wire will actually carry.
            assert!(verify(
                &public,
                transaction.digest(),
                &signature_in(&transaction.signed(&key())),
            ));
        }
    }

    /// Every field has to reach the digest, and a vector pins only the field
    /// values it happens to hold: a field dropped from the element list still
    /// produces a stable, plausible digest for every transaction that shares
    /// that field's value.
    ///
    /// So each field is moved on its own and the digest must move with it. The
    /// deployment is in the same loop because the chain id is an element like
    /// any other and is the one whose absence is invisible until an order is
    /// replayed on the wrong exchange.
    #[test]
    fn every_field_a_transaction_carries_reaches_its_digest() {
        let of = |zone, order: &NewOrder| {
            hex(&create_order(zone, order)
                .expect("a buildable order")
                .digest()
                .to_bytes())
        };
        let base = of(Zone::Testnet, &BUY);
        let mut moved: Vec<(&str, String)> = vec![("deployment", of(Zone::Mainnet, &BUY))];
        let mut push = |name: &'static str, order: NewOrder| {
            moved.push((name, of(Zone::Testnet, &order)));
        };
        push(
            "account",
            NewOrder {
                account: BUY.account + 1,
                ..BUY
            },
        );
        push(
            "api key",
            NewOrder {
                api_key: BUY.api_key + 1,
                ..BUY
            },
        );
        push(
            "market",
            NewOrder {
                market: BUY.market + 1,
                ..BUY
            },
        );
        push(
            "client index",
            NewOrder {
                client_index: BUY.client_index + 1,
                ..BUY
            },
        );
        push(
            "base amount",
            NewOrder {
                base_amount: BUY.base_amount + 1,
                ..BUY
            },
        );
        push(
            "price",
            NewOrder {
                price: BUY.price + 1,
                ..BUY
            },
        );
        push(
            "side",
            NewOrder {
                ask: !BUY.ask,
                ..BUY
            },
        );
        push(
            "reduce only",
            NewOrder {
                reduce_only: !BUY.reduce_only,
                ..BUY
            },
        );
        push(
            "expiry",
            NewOrder {
                expiry_ms: BUY.expiry_ms + 1,
                ..BUY
            },
        );
        push(
            "deadline",
            NewOrder {
                deadline_ms: BUY.deadline_ms + 1,
                ..BUY
            },
        );
        push(
            "nonce",
            NewOrder {
                nonce: BUY.nonce + 1,
                ..BUY
            },
        );

        for (name, digest) in &moved {
            assert_ne!(*digest, base, "the {name} does not reach the digest");
        }
        // And no two of them collide, which is what a field written into the
        // wrong slot of the element list would produce.
        let mut seen: Vec<&String> = moved.iter().map(|(_, digest)| digest).collect();
        seen.sort();
        let all = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), all, "two different fields hash to one digest");

        // The same for a cancel's four of its own.
        let cancel = |cancel: &Cancel| {
            hex(&cancel_order(Zone::Testnet, cancel)
                .expect("a buildable cancel")
                .digest()
                .to_bytes())
        };
        let resting = cancel(&PULL);
        for moved in [
            Cancel {
                account: PULL.account + 1,
                ..PULL
            },
            Cancel {
                api_key: PULL.api_key + 1,
                ..PULL
            },
            Cancel {
                market: PULL.market + 1,
                ..PULL
            },
            Cancel {
                index: PULL.index + 1,
                ..PULL
            },
            Cancel {
                deadline_ms: PULL.deadline_ms + 1,
                ..PULL
            },
            Cancel {
                nonce: PULL.nonce + 1,
                ..PULL
            },
        ] {
            assert_ne!(cancel(&moved), resting);
        }
        // A cancel is not the order it cancels, which is the transaction type
        // doing its job: every other element these two share is equal.
        assert_ne!(resting, base);
    }

    /// A negative field is refused by name rather than hashed.
    ///
    /// The venue casts each field straight into a Goldilocks element, so a -1
    /// becomes an element near 2^64 rather than a small one — and this module
    /// reduces where theirs does not, so the two would disagree and the venue
    /// would recover a stranger. Nothing here is ever legitimately negative, so
    /// the value is refused where it can still be read.
    #[test]
    fn a_negative_field_is_refused_rather_than_signed_as_a_huge_one() {
        let refused = |order: NewOrder| {
            create_order(Zone::Testnet, &order).expect_err("a negative field is not signable")
        };
        assert_eq!(
            refused(NewOrder { nonce: -1, ..BUY }),
            SignError::Field("nonce"),
        );
        assert_eq!(
            refused(NewOrder { market: -1, ..BUY }),
            SignError::Field("market index"),
        );
        assert_eq!(
            refused(NewOrder {
                base_amount: -1,
                ..BUY
            }),
            SignError::Field("base amount"),
        );
        assert_eq!(
            cancel_order(Zone::Testnet, &Cancel { index: -1, ..PULL })
                .expect_err("a negative index is not signable"),
            SignError::Field("order index"),
        );
        // And the sentence names the field, because "invalid transaction" sends
        // a reader to read the whole thing.
        assert!(
            SignError::Field("base amount")
                .to_string()
                .contains("base amount")
        );
    }

    /// Past every format gate the venue has, with nothing behind the last one.
    ///
    /// What 29500 discriminates, exactly: the venue parsed four
    /// colon-separated fields, accepted the deadline as a number inside its
    /// own window, accepted both indices, and unhexed 80 bytes — then failed
    /// the signature. A token this module cannot spell right answers 20013
    /// instead, so 29500 is evidence about the token's *shape*, and that is
    /// all it is.
    ///
    /// It is not evidence that the signature is any good. The key is
    /// registered on no account, so a correct signature and 80 random bytes
    /// both land on 29500; nothing reachable without a registered key tells
    /// them apart. The cryptography is pinned offline instead, against the
    /// venue's own signer, by `the_digest_matches_the_official_signer`,
    /// `the_public_key_derives_from_the_private_key` and
    /// `a_signature_from_the_official_signer_verifies`. This test covers the
    /// one thing those cannot: that the venue reads what this module writes.
    ///
    /// Ignored: it is the only test that touches the network, and it opens its
    /// own connection so it does not depend on any other test having run. It
    /// is a GET; nothing in this module can place an order or move funds.
    #[test]
    #[ignore = "hits the live venue"]
    fn a_well_formed_token_reaches_the_signature_check() {
        let token = auth_token(&key(), ACCOUNT, API_KEY, now() + 600).expect("inside the window");

        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(15)))
            // The venue puts its reason in the body of a 4xx, and ureq would
            // otherwise drop the body and report the bare status.
            .http_status_as_error(false)
            .build()
            .into();
        let mut response = agent
            .get(&format!(
                "https://mainnet.zklighter.elliot.ai/api/v1/accountActiveOrders\
                 ?account_index={ACCOUNT}&market_id=1"
            ))
            .header("Authorization", &token)
            .call()
            .expect("Lighter reachable");
        let body: serde_json::Value = response.body_mut().read_json().expect("a JSON refusal");

        assert_eq!(
            body.get("code").and_then(serde_json::Value::as_i64),
            Some(29500),
            "expected the signature check, got {body}"
        );
    }
}
