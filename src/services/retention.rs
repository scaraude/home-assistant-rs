//! Background service that periodically downsamples historical readings.
//!
//! Delegates the actual work to [`Database::run_retention`], which keeps raw
//! data for the last week and downsamples everything older (see that module for
//! the policy). Runs shortly after startup and then once per day.

use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info};

use crate::db::Database;

/// Wait a little after boot before the first sweep so startup I/O settles.
const STARTUP_DELAY: Duration = Duration::from_secs(60);
/// Run the rollup once per day.
const INTERVAL: Duration = Duration::from_secs(24 * 3_600);

pub struct RetentionService {
    db: Arc<Database>,
}

impl RetentionService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn run(self) {
        info!("RetentionService started");
        tokio::time::sleep(STARTUP_DELAY).await;

        loop {
            let now = chrono::Utc::now().timestamp();
            let db = self.db.clone();

            // `run_retention` is a blocking, potentially long sweep (large
            // backlog on first run) that holds the sync DB mutex, so run it on
            // the blocking pool rather than the async runtime.
            match tokio::task::spawn_blocking(move || db.run_retention(now)).await {
                Ok(Ok(stats)) => info!(
                    energy_deleted = stats.energy_deleted,
                    temperature_deleted = stats.temperature_deleted,
                    chunks = stats.chunks_processed,
                    "Retention rollup finished"
                ),
                Ok(Err(e)) => error!(error = %e, "Retention rollup failed"),
                Err(e) => error!(error = %e, "Retention task panicked"),
            }

            tokio::time::sleep(INTERVAL).await;
        }
    }
}
