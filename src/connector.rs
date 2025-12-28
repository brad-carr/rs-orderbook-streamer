use crate::broker::{Exchange, SymbolKey};
use core_affinity::CoreId;
use crossbeam_channel::{unbounded, Sender};
use std::thread;

/// Commands sent from the Broker to the pinned Exchange Connector.
pub enum ConnectorCmd {
    Subscribe(SymbolKey),
    Unsubscribe(SymbolKey),
}

/// Manages pinned worker threads for exchange connectivity.
pub struct ExchangeConnector {
    cmd_tx: Sender<ConnectorCmd>,
}

impl ExchangeConnector {
    /// Spawns a worker thread pinned to a specific CPU core.
    ///
    /// # Performance
    /// * **Core Pinning**: Uses `core_affinity` to prevent OS context switching.
    /// * **Busy-Waiting**: In a production hot-path, the receiver would loop
    ///   with `spin_loop` to minimize wake-up latency.
    pub fn new(core_id: CoreId) -> Self {
        let (tx, rx) = unbounded::<ConnectorCmd>();

        thread::spawn(move || {
            // Pin this thread to the specified core
            core_affinity::set_for_current(core_id);

            for cmd in rx {
                match cmd {
                    ConnectorCmd::Subscribe(key) => {
                        Self::handle_physical_subscribe(key);
                    }
                    ConnectorCmd::Unsubscribe(key) => {
                        Self::handle_physical_unsubscribe(key);
                    }
                }
            }
        });

        Self { cmd_tx: tx }
    }

    /// Sends a subscription command to the pinned worker.
    pub fn send_cmd(&self, cmd: ConnectorCmd) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn handle_physical_subscribe(key: SymbolKey) {
        // Logic for opening WebSocket/FIX session based on Exchange enum
    }

    fn handle_physical_unsubscribe(key: SymbolKey) {
        // Logic for sending 'unsubscribe' message or closing connection
    }
}