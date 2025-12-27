//! Data structures for L1-resident order book state.

use std::sync::atomic::{AtomicU64, Ordering};

pub const BOOK_DEPTH: usize = 32;
pub const SENTINEL_QTY: i64 = 0;

/// A single price level in the order book.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Level {
    /// Fixed-point price (signed to support spreads).
    pub price: i64,

    /// Fixed-point quantity (negative indicates a marked removal).
    pub qty: i64,
}

/// A cache-aligned, 32-level order book.
///
/// Occupies approximately 1024 bytes, fitting comfortably in L1d cache.
#[repr(C)]
pub struct L1FriendlyBook {
    pub bids: [Level; BOOK_DEPTH],
    pub asks: [Level; BOOK_DEPTH],
    /// Monotonically increasing version for lock-free synchronization.
    pub version: AtomicU64,
}

impl L1FriendlyBook {
    pub fn new() -> Self {
        Self {
            bids: [Level::default(); BOOK_DEPTH],
            asks: [Level::default(); BOOK_DEPTH],
            version: AtomicU64::new(0),
        }
    }

    /// Increments the version counter using Release ordering.
    ///
    /// This signals to the trading engine that a consistent snapshot of the
    /// book is now available in memory. It should be called exactly once
    /// per packet, after all lazy-removals and additions are finalized.
    ///
    /// # Performance
    /// * **Atomic Sync**: Uses `Ordering::Release` to ensure all prior
    ///   memory writes to the `bids` and `asks` arrays are visible to
    ///   other cores performing an `Acquire` load.
    pub fn increment_version(&self) {
        self.version.fetch_add(1, Ordering::Release);
    }

    /// Returns true if the best ask is 0 (uninitialized)
    pub fn asks_empty(&self) -> bool {
        self.asks[0].price == 0
    }

    /// Returns true if the best bid is 0 (uninitialized)
    pub fn bids_empty(&self) -> bool {
        self.bids[0].price == 0
    }

    /// Returns true if both sides are empty
    pub fn is_empty(&self) -> bool {
        self.bids_empty() && self.asks_empty()
    }

    /// Marks a level for lazy deletion by setting a sentinel quantity.
    pub fn mark_removal(side: &mut [Level; BOOK_DEPTH], index: usize) {
        side[index].qty = SENTINEL_QTY;
    }

    /// Compact the array by removing sentinels and shifting levels to the front.
    ///
    /// This method removes all levels marked with [SENTINEL_QTY] by shifting
    /// active levels to the front of the array.
    ///
    /// # Performance
    /// * **Time Complexity**: O(N) where N is `BOOK_DEPTH`.
    /// * **Mechanical Sympathy**: Operates on a single 1KB contiguous block
    ///   to ensure L1 cache-line prefetching is utilized.
    ///
    /// # Examples
    /// ```rust
    /// # use hft_broker::model::{L1FriendlyBook, Level, SENTINEL_QTY};
    /// let mut book = L1FriendlyBook::new();
    /// book.bids[0] = Level { price: 100, qty: SENTINEL_QTY };
    /// book.bids[1] = Level { price: 99, qty: 10 };
    /// L1FriendlyBook::compact(&mut book.bids);
    /// assert_eq!(book.bids[0].price, 99);
    /// ```
    pub fn compact(side: &mut [Level; BOOK_DEPTH]) {
        let mut next_fill = 0;
        for i in 0..BOOK_DEPTH {
            if side[i].qty != SENTINEL_QTY && side[i].price != 0 {
                if i != next_fill {
                    side[next_fill] = side[i];
                }
                next_fill += 1;
            }
        }
        // Clear remaining slots
        for i in next_fill..BOOK_DEPTH {
            side[i] = Level::default();
        }
    }
}