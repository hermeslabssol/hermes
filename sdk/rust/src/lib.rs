//! # logios-client
//!
//! Async Rust client for the [Logios](https://hermes-labs.xyz) read API — the
//! Solana-native chain that writes itself.
//!
//! Logios runs an SVM (Sealevel) runtime where every slot is authored by an
//! autonomous agent that emits a signed decision receipt. This crate gives you
//! typed, `async` access to its public `/v1` endpoints.
//!
//! Everything is Solana-flavoured: base58 blockhashes/signatures, slots,
//! compute units. Sealevel all the way down.
//!
//! ## Example
//!
//! ```no_run
//! use logios_client::LogiosClient;
//!
//! # async fn run() -> Result<(), logios_client::Error> {
//! let logios = LogiosClient::new();
//!
//! let stats = logios.stats().await?;
//! println!("height {} · {} tps", stats.block_height, stats.tps);
//!
//! let block = logios.latest_block().await?;
//! let ex = logios.explain(Some(block.slot)).await?;
//! println!("#{}: {}", ex.slot, ex.narration);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Default API origin (no trailing `/v1`).
pub const DEFAULT_BASE_URL: &str = "https://hermes-labs.xyz";

/// A base58-encoded string (blockhash, signature, or pubkey), as used across
/// the SVM. Carries no runtime guarantee — documents intent only.
pub type Base58String = String;

/// A slot number — the monotonic index of a sealed block on Logios.
pub type Slot = u64;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by [`LogiosClient`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport-level failure (DNS, connection, timeout, TLS).
    #[error("transport error calling {path}: {source}")]
    Transport {
        /// Request path that failed.
        path: String,
        /// Underlying `reqwest` error.
        #[source]
        source: reqwest::Error,
    },

    /// The API responded with a non-success HTTP status.
    #[error("api returned {status} for {path}: {body}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Request path.
        path: String,
        /// Raw response body.
        body: String,
    },

    /// The response body could not be deserialized into the expected type.
    #[error("failed to decode response from {path}: {source}")]
    Decode {
        /// Request path.
        path: String,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// A single-row endpoint returned an empty array.
    #[error("endpoint {path} returned no rows")]
    Empty {
        /// Request path.
        path: String,
    },
}

type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Public types — normalized, Solana-flavoured.
// ---------------------------------------------------------------------------

/// Network-wide counters and genesis info (`GET /v1/stats`).
#[derive(Debug, Clone, Deserialize)]
pub struct Stats {
    /// Height of the latest sealed slot.
    pub block_height: u64,
    /// Total number of decision receipts the agent has authored.
    pub commits: u64,
    /// Recent transactions-per-second throughput.
    pub tps: f64,
    /// Validator set. Logios is single-leader, so this is the literal `"self"`.
    pub validators: String,
    /// When the genesis slot was sealed (RFC-3339).
    pub genesis_at: String,
}

/// A sealed block / slot. Mirrors `GET /v1/block/latest` and `GET /v1/blocks`.
///
/// The wire shape uses `number`/`hash`; serde aliases map them onto the
/// Solana-flavoured field names below.
#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    /// The slot number (wire field: `number`).
    #[serde(rename = "number")]
    pub slot: Slot,
    /// base58 blockhash (wire field: `hash`).
    #[serde(rename = "hash")]
    pub blockhash: Base58String,
    /// Number of transactions packed into the slot.
    pub txns: u64,
    /// Compute units consumed by the slot.
    pub compute_units: u64,
    /// When the slot was sealed (RFC-3339).
    pub sealed_at: String,
}

/// A signed decision receipt (`GET /v1/receipts`).
#[derive(Debug, Clone, Deserialize)]
pub struct Receipt {
    /// The slot this receipt attests to (wire field: `block_number`).
    #[serde(rename = "block_number")]
    pub slot: Slot,
    /// base58 ed25519 signature over the decision (wire field: `hash`).
    #[serde(rename = "hash")]
    pub signature: Base58String,
    /// Human-readable summary of the agent's decision for the slot.
    pub decision: String,
    /// When the receipt was written to the ledger (RFC-3339).
    pub created_at: String,
}

/// Live status of the autonomous leader agent (`GET /v1/agent`).
#[derive(Debug, Clone, Deserialize)]
pub struct AgentStatus {
    /// Current lifecycle phase, e.g. `"VERIFY"`.
    pub status: String,
    /// What the agent is doing right now.
    pub task: String,
    /// Agent build version, e.g. `"v0.4.1"`.
    pub version: String,
    /// When this status was last refreshed (RFC-3339).
    pub updated_at: String,
}

/// A single line from the agent's system log stream (`GET /v1/logs`).
#[derive(Debug, Clone, Deserialize)]
pub struct LogEntry {
    /// Log severity / channel, e.g. `"ok"`, `"info"`, `"decision"`.
    pub level: String,
    /// The log message.
    pub message: String,
    /// When the line was emitted (RFC-3339).
    pub created_at: String,
}

/// A self-shipped protocol update — the chain rewriting itself (`GET /v1/updates`).
#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    /// Agent version that shipped the change.
    pub version: String,
    /// Short title of the change.
    pub title: String,
    /// Longer description of what shipped.
    pub body: String,
    /// When the update went live (RFC-3339).
    pub shipped_at: String,
}

/// A single roadmap entry (`GET /v1/roadmap`).
#[derive(Debug, Clone, Deserialize)]
pub struct RoadmapItem {
    /// Numeric tier (1 = Foundations).
    pub tier: u32,
    /// Human-readable tier name.
    pub tier_name: String,
    /// Delivery status: `"done"`, `"shipping"`, `"queued"`.
    pub status: String,
    /// Subsystem the item belongs to, e.g. `"chain"`, `"vm"`, `"api"`.
    pub section: String,
    /// What the item is.
    pub title: String,
}

/// Plain-English narration of a slot (`POST /v1/explain`).
#[derive(Debug, Clone, Deserialize)]
pub struct Explanation {
    /// Whether the explanation was produced successfully.
    pub ok: bool,
    /// The slot being narrated (wire field: `number`).
    #[serde(rename = "number")]
    pub slot: Slot,
    /// base58 blockhash (wire field: `hash`).
    #[serde(rename = "hash")]
    pub blockhash: Base58String,
    /// Transaction count for the slot.
    pub txns: u64,
    /// The human-readable narration of what happened in the slot.
    pub narration: String,
}

#[derive(Serialize)]
struct ExplainArgs {
    /// PostgREST RPC argument name for the slot to narrate.
    #[serde(skip_serializing_if = "Option::is_none")]
    p_number: Option<Slot>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Async client for the Logios public read API (`/v1`).
///
/// Cheap to clone — the inner [`reqwest::Client`] is reference-counted and
/// pools connections, so prefer cloning over constructing new clients.
#[derive(Debug, Clone)]
pub struct LogiosClient {
    base_url: String,
    http: reqwest::Client,
}

impl Default for LogiosClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LogiosClient {
    /// Create a client pointed at the default origin ([`DEFAULT_BASE_URL`]).
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Create a client pointed at a custom origin (no trailing `/v1`).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("logios-client-rs/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest client builds with default config");
        Self::with_client(base_url, http)
    }

    /// Create a client from a pre-configured [`reqwest::Client`] (custom
    /// timeouts, proxies, TLS, etc.).
    pub fn with_client(base_url: impl Into<String>, http: reqwest::Client) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self { base_url, http }
    }

    /// Network-wide counters and genesis info. Maps to `GET /v1/stats`.
    pub async fn stats(&self) -> Result<Stats> {
        self.get_one("/v1/stats").await
    }

    /// The most recently sealed block. Maps to `GET /v1/block/latest`.
    pub async fn latest_block(&self) -> Result<Block> {
        self.get_one("/v1/block/latest").await
    }

    /// A window of recent blocks, newest first. Maps to `GET /v1/blocks`.
    pub async fn blocks(&self, limit: Option<u32>) -> Result<Vec<Block>> {
        self.get_list("/v1/blocks", limit).await
    }

    /// Recent decision receipts, newest first. Maps to `GET /v1/receipts`.
    pub async fn receipts(&self, limit: Option<u32>) -> Result<Vec<Receipt>> {
        self.get_list("/v1/receipts", limit).await
    }

    /// The autonomous agent's current status. Maps to `GET /v1/agent`.
    pub async fn agent(&self) -> Result<AgentStatus> {
        self.get_one("/v1/agent").await
    }

    /// Recent system-log lines, newest first. Maps to `GET /v1/logs`.
    pub async fn logs(&self, limit: Option<u32>) -> Result<Vec<LogEntry>> {
        self.get_list("/v1/logs", limit).await
    }

    /// Recent self-shipped updates, newest first. Maps to `GET /v1/updates`.
    pub async fn updates(&self, limit: Option<u32>) -> Result<Vec<Update>> {
        self.get_list("/v1/updates", limit).await
    }

    /// The full roadmap, ordered by tier. Maps to `GET /v1/roadmap`.
    pub async fn roadmap(&self) -> Result<Vec<RoadmapItem>> {
        self.get_list("/v1/roadmap", None).await
    }

    /// Plain-English narration of a slot. Maps to `POST /v1/explain`.
    ///
    /// Pass `None` to narrate the latest sealed slot.
    pub async fn explain(&self, slot: Option<Slot>) -> Result<Explanation> {
        let path = "/v1/explain";
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .json(&ExplainArgs { p_number: slot })
            .send()
            .await
            .map_err(|source| Error::Transport {
                path: path.to_string(),
                source,
            })?;
        let body = read_ok(resp, path).await?;
        serde_json::from_str(&body).map_err(|source| Error::Decode {
            path: path.to_string(),
            source,
        })
    }

    // -- internals ----------------------------------------------------------

    /// GET a list endpoint, decoding the JSON array. Honours an optional `limit`.
    async fn get_list<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        limit: Option<u32>,
    ) -> Result<Vec<T>> {
        let mut url = format!("{}{}", self.base_url, path);
        if let Some(n) = limit {
            let clamped = n.clamp(1, 1000);
            url.push_str(&format!("?limit={clamped}"));
        }
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|source| Error::Transport {
                path: path.to_string(),
                source,
            })?;
        let body = read_ok(resp, path).await?;
        serde_json::from_str(&body).map_err(|source| Error::Decode {
            path: path.to_string(),
            source,
        })
    }

    /// GET a list endpoint and return its first row, erroring if empty.
    async fn get_one<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let rows: Vec<T> = self.get_list(path, None).await?;
        rows.into_iter().next().ok_or_else(|| Error::Empty {
            path: path.to_string(),
        })
    }
}

/// Read a response body, mapping non-success statuses into [`Error::Api`].
async fn read_ok(resp: reqwest::Response, path: &str) -> Result<String> {
    let status = resp.status();
    let body = resp.text().await.map_err(|source| Error::Transport {
        path: path.to_string(),
        source,
    })?;
    if !status.is_success() {
        return Err(Error::Api {
            status: status.as_u16(),
            path: path.to_string(),
            body,
        });
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_is_normalized() {
        let c = LogiosClient::with_base_url("https://example.test///");
        assert_eq!(c.base_url, "https://example.test");
    }

    #[test]
    fn block_deserializes_from_wire_shape() {
        let json = r#"{
            "number": 4542502,
            "hash": "hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky",
            "txns": 130,
            "compute_units": 48000000,
            "sealed_at": "2026-06-06T16:20:00.01422+00:00"
        }"#;
        let b: Block = serde_json::from_str(json).expect("decodes");
        assert_eq!(b.slot, 4542502);
        assert_eq!(b.compute_units, 48_000_000);
        assert!(b.blockhash.starts_with("hz67"));
    }

    #[test]
    fn receipt_maps_block_number_and_hash() {
        let json = r#"{
            "block_number": 4542502,
            "hash": "NE7rwDLofnBsaEFm73dD",
            "decision": "sealed 130 txns",
            "created_at": "2026-06-06T16:20:00.01422+00:00"
        }"#;
        let r: Receipt = serde_json::from_str(json).expect("decodes");
        assert_eq!(r.slot, 4542502);
        assert_eq!(r.signature, "NE7rwDLofnBsaEFm73dD");
    }

    #[test]
    fn explain_args_omit_none() {
        let s = serde_json::to_string(&ExplainArgs { p_number: None }).unwrap();
        assert_eq!(s, "{}");
        let s = serde_json::to_string(&ExplainArgs {
            p_number: Some(4542500),
        })
        .unwrap();
        assert_eq!(s, r#"{"p_number":4542500}"#);
    }
}
