use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::model::L1FriendlyBook;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Represents the specific instrument class.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum ProductType {
    Spot,
    Future,
    Perpetual,
    VanillaOption, // Avoids collision with core::option::Option
}

/// Supported exchange venues.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Exchange {
    Binance,
    Coinbase,
    Kraken,
}

/// A unique identifier for a market data stream.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct SymbolKey {
    pub exchange: Exchange,
    pub symbol: String, // e.g., "BTC-USDT"
    pub product: ProductType,
}

/// Trait for handling subscription teardown logic.
pub trait SubscriptionTeardown: Send + Sync {
    fn teardown(&self, key: &SymbolKey);
}

/// Manages shared book states and subscription reference counting.
#[derive(Clone)]
pub struct MarketBroker {
    /// Maps symbols to their L1-resident book and active handle count.
    subscriptions: Arc<RwLock<HashMap<SymbolKey, Arc<SubscriptionData>>>>,
}

struct SubscriptionData {
    book: Arc<L1FriendlyBook>,
    ref_count: Arc<AtomicUsize>,
}

/// An RAII handle that decrements the reference count when dropped.
///
/// When the last handle for a symbol is dropped, the broker can
/// trigger an unsubscription from the exchange.
pub struct SubscriptionHandle {
    pub key: SymbolKey,
    pub book: Arc<L1FriendlyBook>,
    registry: Arc<RwLock<HashMap<SymbolKey, Arc<SubscriptionData>>>>,
    teardown: Box<dyn SubscriptionTeardown>,
}

impl MarketBroker {
    /// Creates a new broker instance.
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribes to a specific market product.
    ///
    /// If this is the first subscription for a given `SymbolKey`, it initiates the subscription
    /// process with the exchange, otherwise it shares the existing subscription.
    ///
    /// Returns an RAII handle to the shared L1-resident book.
    pub fn subscribe(
        &self,
        exchange: Exchange,
        symbol: &str,
        product: ProductType
    ) -> SubscriptionHandle {
        let key = SymbolKey {
            exchange,
            symbol: symbol.to_string(),
            product,
        };

        let mut subs = self.subscriptions.write();

        // Entry API handles the atomic check-and-insert
        let data = subs.entry(key.clone()).or_insert_with(|| {
            Arc::new(SubscriptionData {
                book: Arc::new(L1FriendlyBook::new()),
                ref_count: Arc::new(AtomicUsize::new(0)),
            })
        });

        // If the previous value was 0, this is the first active handle.
        if data.ref_count.fetch_add(1, Ordering::SeqCst) == 0 {
            self.initiate_subscription(&key);
        }

        SubscriptionHandle {
            key,
            book: Arc::clone(&data.book),
            registry: Arc::clone(&self.subscriptions),
            teardown: Box::new(self.clone()),
        }
    }

    fn initiate_subscription(&self, key: &SymbolKey) {
        todo!()
    }

    fn terminate_subscription(&self, key: &SymbolKey) {
        todo!()
    }
}

impl SubscriptionTeardown for MarketBroker {
    fn teardown(&self, key: &SymbolKey) {
        self.terminate_subscription(key);
    }
}

impl Drop for SubscriptionHandle {
    /// Decrements the reference count and performs cleanup.
    fn drop(&mut self) {
        let mut subs = self.registry.write();
        if let Some(data) = subs.get(&self.key) {
            if data.ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
                subs.remove(&self.key);
                self.teardown.teardown(&self.key);
            }
        }
    }
}