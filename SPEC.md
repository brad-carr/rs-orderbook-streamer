# Specification: High-Performance L2 Orderbook Broker (Rust)

## 1. Project Goal
A low-latency market data broker managing exchange connections and hardware resources. It provides a homogenized, versioned view of L2 orderbooks to an external trading engine using atomic signaling and L1-optimized memory layouts.

## 2. Core Architecture: Colocated Pipeline
* **Run-to-Completion:** Each pinned core handles the entire lifecycle of a stream: I/O (WebSocket ingestion), Parsing (Filtering/Zero-copy), and State Update (Orderbook).
* **Zero-Copy Ingress:** Raw wire-bytes are streamed into a pre-allocated **Ring Buffer**.
* **High-Pass Filtering:** The parser processes incoming exchange streams (even those with 1,000+ levels) in-place. Levels outside the current "Top 32" price range are immediately discarded to prevent cache pollution.
* **Communication:** Atomic Versioning notifies the engine of updates, bypassing kernel-level synchronization.



## 3. Resource Orchestration (The Bitmask Contract)
* **Initialization:** Broker is initialized with a `u64` bitmask (e.g., `0b1100`).
* **Affinity:** One worker thread per "set" bit is spawned and pinned using `core_affinity`.
* **Scaling:** Each core operates as an independent "Pipeline Unit," managing a subset of assigned streams to eliminate inter-core "hop" latency.

## 4. Driver & Plugin System
* **ExchangeID:** Unique identifier per exchange (Binance, Coinbase, etc.).
* **ExchangeDriver Trait:** Defines subscription wire-protocols and the `parse_message(&[u8])` hook.
* **Zero-FP Parsing:** Custom byte-iterators convert numeric strings directly to `i64` fixed-point, avoiding FPU overhead and non-determinism.

## 5. Subscription Management (The Registrar)
* **Reference Counting:** Tracks multi-consumer interest in a `(Exchange, Symbol)` pair.
* **RAII Handle:** Returns a `Subscription` object; dropping the handle decrements the count.
* **Auto-Cleanup:** Automatically sends unsubscription messages to the exchange when the last consumer for a symbol exits.

## 6. Data Model (L1-Optimized L2)
To achieve nanosecond-level determinism, the "Hot Path" is sculpted for mechanical sympathy with the CPU's L1 cache.

* **Numeric Representation:** * **Price:** `i64` (Signed to support synthetic/spread instruments).
    * **Quantity:** `i64` (Must be $\ge 0$).
* **Scaling Metadata:** Each `Subscription` exposes `price_exponent` and `qty_exponent` as `i8`. Actual value is $i64 \times 10^{Exp}$.
* **L1 Storage Strategy:** * Uses a flat, contiguous `#[repr(C)]` array of **32 Bids** and **32 Asks** (~1KB total).
    * Fits entirely within a standard 32KB L1d cache, allowing a "single sweep" read.
* **Lazy Invalidation (Mark and Sweep):**
    * **Phase 1 (Mark):** When an update signals a removal (`qty == 0`), the parser marks the price level with a sentinel value ($Qty = -1$). This is an $O(1)$ operation.
    * **Phase 2 (Sweep):** To minimize $O(N)$ memory shifts, the array is only compacted once per packet processing completion, or only when an "Add" operation requires a cleared slot.
* **Signaling:** An `AtomicU64` version counter is updated once the entire packet (and any required compaction) is finalized.



## 7. ADR: Colocated vs. Separated Model
**Decision:** We favor a **Colocated (Run-to-Completion)** model over a Separated (Producer-Consumer) model.
* **Reasoning:** In modern HFT, the "Cross-Core Penalty" (L3/Mesh hop latency, ~40-100ns) often exceeds the time required to scan and filter an L2 update.
* **Depth Handling:** Even for updates containing thousands of levels (e.g., Coinbase Prime), our parser filters for the "Top 32" in-place. It is more efficient to discard deep-book data on a single core than to pay the latency tax of a cross-core handoff.

## 8. Project Structure
```text
├── Cargo.toml          # Performance-tuned profile (LTO, codegen-units=1)
├── SPEC.md             # This document
├── src/
│   ├── lib.rs          # Module declarations & public API
│   ├── broker.rs       # Main Broker, Subscription & Ref-Counting
│   ├── driver/
│   │   ├── mod.rs      # Driver Trait definition
│   │   └── binance.rs  # Example implementation
│   ├── model.rs        # L1FriendlyBook, Fixed-Point & Atomic State
│   ├── connector.rs    # Pinned Runtimes & WebSocket handlers
│   └── util.rs         # Fast byte-iterator parsers
└── tests/
    └── latency.rs      # Core-Isolation & jitter benchmarks