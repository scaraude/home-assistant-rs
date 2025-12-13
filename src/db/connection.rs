use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use tracing::{debug, error, info};

/// Extension trait for Mutex to handle poisoned mutexes gracefully.
///
/// Instead of panicking on poisoned mutex (which happens when a thread panics while holding
/// the lock), this trait provides a method that recovers the data and logs an error.
/// This is acceptable for our use case because:
/// 1. The mutex protects SQLite connection which has its own consistency guarantees
/// 2. Partial writes are prevented by transactions
/// 3. Better to continue serving requests than to panic the entire service
pub trait MutexExt<T> {
    /// Lock the mutex, recovering from poison if necessary
    fn lock_or_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    fn lock_or_recover(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poison_error) => {
                error!("Mutex was poisoned (recovered from panic) - recovering data anyway");
                poison_error.into_inner()
            }
        }
    }
}

/// RAII transaction guard that automatically rolls back on drop unless explicitly committed.
///
/// This prevents partial writes when errors occur and ensures database consistency.
/// The transaction is rolled back automatically when the guard is dropped, unless
/// commit() is called first.
///
/// # Example
/// ```
/// let tx = Transaction::begin(&conn)?;
/// // ... perform operations ...
/// tx.commit()?; // Explicit commit
/// // If commit() is not called, transaction is automatically rolled back on drop
/// ```
pub struct Transaction<'a> {
    pub conn: &'a Connection,
    committed: bool,
}

impl<'a> Transaction<'a> {
    /// Begin a new transaction
    pub fn begin(conn: &'a Connection) -> Result<Self> {
        conn.execute("BEGIN TRANSACTION", [])?;
        debug!("Transaction started");
        Ok(Transaction {
            conn,
            committed: false,
        })
    }

    /// Commit the transaction
    pub fn commit(mut self) -> Result<()> {
        self.conn.execute("COMMIT", [])?;
        self.committed = true;
        debug!("Transaction committed");
        Ok(())
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Rollback on drop (error or panic)
            if let Err(e) = self.conn.execute("ROLLBACK", []) {
                error!(error = %e, "Failed to rollback transaction on drop");
            } else {
                tracing::warn!("Transaction rolled back (not committed)");
            }
        }
    }
}

/// Thread-safe database connection wrapper
pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Create or open the database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        info!(path = %path_str, "Initializing database connection");

        // Ensure the parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            let parent_str = parent.display().to_string();
            debug!(parent_dir = %parent_str, "Checking parent directory");

            match fs::create_dir_all(parent) {
                Ok(_) => debug!(parent_dir = %parent_str, "Parent directory exists or created"),
                Err(e) => {
                    error!(error = %e, parent_dir = %parent_str, "Failed to create parent directory");
                    return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e)));
                }
            }
        }

        debug!(path = %path_str, "Opening database connection");
        let conn = match Connection::open(&path) {
            Ok(c) => {
                info!(path = %path_str, "Database connection opened successfully");
                c
            }
            Err(e) => {
                error!(error = %e, path = %path_str, "Failed to open database connection");
                return Err(e);
            }
        };

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Initialize schema (this is defined in schema.rs via impl Database)
        db.init_schema()?;

        info!(path = %path_str, "Database initialization complete");
        Ok(db)
    }
}
