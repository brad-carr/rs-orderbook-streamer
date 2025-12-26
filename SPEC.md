# Specification: High-Performance L2 Orderbook Broker (Rust)

## 1. Project Goal
A low-latency market data broker that manages exchange connections and hardware resources. It provides a homogenized, versioned view of L2 orderbooks to an external trading engine using atomic signaling to eliminate kernel-level synchronization.

## 2. Core Architecture: The Pipeline
* **I/O & Processing Layer:** Decoupled architecture using dedicated `Tokio` runtimes pinned to specific physical cores.
* **Driver-Based Plugins:** Modular exchange drivers handle protocol-specific handshakes and zero-copy parsing.
* **Communication:** Atomic Versioning (Dirty Flags) notifies the trading engine of updates, bypassing kernel context switches.



## 3. Resource Orchestration (The Bitmask Contract)
* **Initialization:** The Broker is initialized with a `u64` bitmask.
* **Affinity:** One worker thread is spawned for every "set" bit in the mask.
* **Pinning:** Each thread is pinned to its corresponding `CoreID` using `core_affinity`.
* **Isolation:** Cores not present in the mask are strictly reserved for the trading engine.



## 4. Driver & Plugin System
To support extensibility, the library utilizes a `Trait`-based system:
* **ExchangeID:** A unique identifier for each supported exchange.
* **ExchangeDriver Trait:** Exposes methods for `build_subscription_msg` and `parse_message(&[u8])`.
* **Fast Parsing:** Implements a zero-copy, byte-iterator parser to convert numeric strings directly to `i64` fixed-point integers, avoiding the FPU.

## 5. Subscription Management (The Registrar)
The Broker acts as a stateful broker for stream interests:
* **Reference Counting:** Tracks how many consumers are interested in a specific `(Exchange, Symbol)` pair.
* **Subscription Handle:** Returns a RAII-based handle to the engine.
* **Auto-Cleanup:** When the last handle for a symbol is dropped, the library automatically sends an unsubscription message and closes the network task.


## 6. Data Model (L1-Optimized L2)
To ensure nanosecond determinism, the "Hot Path" is sculpted for the CPU cache.
* **Numeric Representation:** `i64` Fixed-Point. Price is signed (supports spreads); Quantity is $\ge 0$.
* **Scaling:** Each sub exposes `i8` exponents ($Value = i64 \times 10^{Exp}$).
* **L1 Strategy:** Uses a flat, contiguous `#[repr(C)]` array of 32 Bids and 32 Asks.
* **Size:** The ~1KB book fits entirely within the 32KB L1d cache.
* **Signaling:** `AtomicU64` versioning for non-blocking state checks.


## 7. Project Structure
```text
├── Cargo.toml          # Performance-tuned dependencies
├── SPEC.md             # This document
├── src/
│   ├── lib.rs          # Module declarations
│   ├── broker.rs       # Main Broker, Subscription logic, & Ref-Counting
│   ├── driver/
│   │   ├── mod.rs      # Driver Trait definition
│   │   ├── binance.rs  # Binance implementation
│   │   └── coinbase.rs # Coinbase implementation
│   ├── model.rs        # L2Book, Fixed-Point (i64), & Atomic State
│   ├── connector.rs    # Pinned Tokio Runtimes & WS Logic
│   └── util.rs         # Fast byte-iterator parsers
└── tests/
    └── latency.rs      # Performance & Core-Isolation benchmarks