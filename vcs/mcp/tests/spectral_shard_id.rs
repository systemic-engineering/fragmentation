//! T4 RED — `ShardId` is now a [`SpectralUuid`] newtype.
//!
//! Per the CRDT spec (`reality-shard-as-crdt.md`) + T4 brief. The
//! shard identifier is no longer `uuid::Uuid v4` (random). It is a
//! `SpectralUuid` — 48 bits active (the quantized spectral coord)
//! + 80 bits dark (the content hash prefix). The empty shard has
//! a canonical address `ShardId::EMPTY = SpectralUuid::EMPTY`.
//!
//! What this file pins:
//!
//! 1. `ShardId::EMPTY` is byte-stable and deterministic.
//! 2. Two empty shards opened back-to-back have the SAME ShardId
//!    — the deduplication property (the CRDT C8 implication).
//! 3. `ShardId::from_content(hash, active)` derives deterministic
//!    IDs from content + spectral position.
//! 4. The wire shard_id is still a 36-char hyphenated string.
//! 5. `ShardId::parse` round-trips through the new Display.
//! 6. The `uuid` crate is no longer in the dispatch path — the
//!    wire-side shard handle is `SpectralUuid::to_string()`.

use fragmentation_mcp::{BudgetMb, Mcp, ShardId, ShardRegistry};

// ---------------------------------------------------------------------------
// EMPTY — the canonical empty-shard address.
// ---------------------------------------------------------------------------

#[test]
fn shard_id_empty_is_deterministic() {
    // The CRDT spec's bottom element: two reads of EMPTY must be
    // byte-identical.
    assert_eq!(ShardId::EMPTY, ShardId::EMPTY);
    assert_eq!(ShardId::EMPTY.to_string(), ShardId::EMPTY.to_string());
}

#[test]
fn shard_id_empty_display_is_36_chars() {
    let s = ShardId::EMPTY.to_string();
    assert_eq!(s.len(), 36, "got {s}");
}

#[test]
fn shard_id_empty_round_trips_parse() {
    let s = ShardId::EMPTY.to_string();
    let parsed = ShardId::parse(&s).expect("parse EMPTY");
    assert_eq!(parsed, ShardId::EMPTY);
}

// ---------------------------------------------------------------------------
// Two empty shards opened in sequence share the same ShardId.
// ---------------------------------------------------------------------------

#[test]
fn two_empty_shards_share_the_same_id() {
    // The load-bearing T4 acceptance: open() with no content must
    // return ShardId::EMPTY (the canonical address). Two sequential
    // opens return the same id — the deduplication property.
    let reg = ShardRegistry::new();
    let id1 = reg.open(BudgetMb(8)).expect("open 1");
    let id2 = reg.open(BudgetMb(64)).expect("open 2"); // different budget; same EMPTY content
    assert_eq!(
        id1, id2,
        "two empty shards must share the canonical EMPTY id (CRDT semilattice bottom)"
    );
    assert_eq!(id1, ShardId::EMPTY);
}

// ---------------------------------------------------------------------------
// ShardId is byte-stable through the wire (open → Display → parse).
// ---------------------------------------------------------------------------

#[test]
fn shard_id_round_trips_through_wire() {
    let mcp = Mcp::new();
    let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fragmentation.shard.open","arguments":{"budget_mb":8}}}"#;
    let response = mcp.dispatch_line(line);
    let value = serde_json::to_value(&response).expect("serialize");
    let shard_id_str = value
        .get("result")
        .and_then(|r| r.get("shard_id"))
        .and_then(|s| s.as_str())
        .expect("shard_id in result")
        .to_string();
    // 36 chars, hyphenated, parseable.
    assert_eq!(shard_id_str.len(), 36);
    let parsed = ShardId::parse(&shard_id_str).expect("parse wire shard_id");
    assert_eq!(parsed.to_string(), shard_id_str);
    // First wire-driven open returns ShardId::EMPTY (no content).
    assert_eq!(parsed, ShardId::EMPTY);
}

#[test]
fn shard_id_parse_rejects_garbage() {
    assert!(ShardId::parse("").is_err());
    assert!(ShardId::parse("not-a-uuid").is_err());
    assert!(ShardId::parse("12345").is_err());
}

// ---------------------------------------------------------------------------
// from_content — derive a ShardId from content_oid + spectral active.
// ---------------------------------------------------------------------------

#[test]
fn shard_id_from_content_is_deterministic() {
    // Same content_hash + same active → same ShardId.
    let hash = [0x42u8; 32];
    let a = ShardId::from_content(0, &hash);
    let b = ShardId::from_content(0, &hash);
    assert_eq!(a, b);
}

#[test]
fn shard_id_from_content_differs_for_different_hashes() {
    let h1 = [0x11u8; 32];
    let h2 = [0x22u8; 32];
    let a = ShardId::from_content(0, &h1);
    let b = ShardId::from_content(0, &h2);
    assert_ne!(a, b);
}

#[test]
fn shard_id_from_content_with_zero_active_and_blake3_empty_equals_empty() {
    // The structural identity: from_content with active=0 and the
    // BLAKE3-of-empty hash should produce EMPTY. This is the
    // homomorphism identity element law (the spec's §2 §3 wired
    // together).
    let blake3_empty: [u8; 32] = [
        // af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
        0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40,
        0x4d, 0xea, 0x36, 0xdc, 0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9,
        0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca, 0xe4, 0x1f,
        0x32, 0x62,
    ];
    let id = ShardId::from_content(0, &blake3_empty);
    assert_eq!(id, ShardId::EMPTY);
}
