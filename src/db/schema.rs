use super::connection::{Database, MutexExt};
use rusqlite::Result;
use tracing::{debug, error, info};

impl Database {
    /// Initialize the database schema
    pub fn init_schema(&self) -> Result<()> {
        debug!("Acquiring database lock for schema initialization");
        let conn = self.conn.lock_or_recover();

        // Create temperature_readings table
        self.create_temperature_readings_table(&conn)?;

        // Create devices table
        self.create_devices_table(&conn)?;

        // Create automation tables
        self.create_automation_tables(&conn)?;

        info!("Database schema initialization complete");
        Ok(())
    }

    /// Create temperature_readings table and indexes
    fn create_temperature_readings_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating temperature_readings table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS temperature_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL,
                battery INTEGER,
                link_quality INTEGER,
                timestamp INTEGER NOT NULL
            )",
            [],
        ) {
            Ok(_) => debug!("Temperature readings table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create temperature_readings table");
                return Err(e);
            }
        }

        // Index for efficient queries by sensor and time
        debug!("Creating index idx_sensor_time if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sensor_time
             ON temperature_readings(device_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_sensor_time created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_sensor_time");
                return Err(e);
            }
        }

        // Migration: Add link_quality column if it doesn't exist
        self.migrate_add_link_quality_column(conn)?;

        Ok(())
    }

    /// Migration: Add link_quality column if it doesn't exist
    fn migrate_add_link_quality_column(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Checking if link_quality column exists");
        let column_exists: Result<i32, _> = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('temperature_readings') WHERE name='link_quality'",
            [],
            |row| row.get(0),
        );

        match column_exists {
            Ok(0) => {
                info!("link_quality column does not exist, adding it");
                match conn.execute(
                    "ALTER TABLE temperature_readings ADD COLUMN link_quality INTEGER",
                    [],
                ) {
                    Ok(_) => info!("Successfully added link_quality column"),
                    Err(e) => {
                        error!(error = %e, "Failed to add link_quality column");
                        return Err(e);
                    }
                }
            }
            Ok(_) => debug!("link_quality column already exists"),
            Err(e) => {
                error!(error = %e, "Failed to check if link_quality column exists");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Create devices table and indexes
    fn create_devices_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating devices table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                mqtt_topic TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                device_type TEXT NOT NULL,
                power_source TEXT NOT NULL,
                added_at INTEGER NOT NULL
            )",
            [],
        ) {
            Ok(_) => debug!("Devices table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create devices table");
                return Err(e);
            }
        }

        // Create index on mqtt_topic for fast lookups
        debug!("Creating index idx_mqtt_topic if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_mqtt_topic ON devices(mqtt_topic)",
            [],
        ) {
            Ok(_) => debug!("Index idx_mqtt_topic created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_mqtt_topic");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Create automation-related tables and indexes
    fn create_automation_tables(&self, conn: &rusqlite::Connection) -> Result<()> {
        // Create automation_rules table
        debug!("Creating automation_rules table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                enabled INTEGER DEFAULT 1,
                condition_operator TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_triggered_at INTEGER,
                trigger_count INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Create automation_conditions table
        debug!("Creating automation_conditions table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_conditions (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                field TEXT NOT NULL,
                operator TEXT NOT NULL,
                value REAL NOT NULL,
                FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_condition_rule
             ON automation_conditions(rule_id)",
            [],
        )?;

        // Create automation_actions table
        debug!("Creating automation_actions table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_actions (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                action TEXT NOT NULL,
                FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_action_rule
             ON automation_actions(rule_id)",
            [],
        )?;

        // Create automation_execution_log table
        debug!("Creating automation_execution_log table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_execution_log (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                rule_name TEXT NOT NULL,
                success INTEGER NOT NULL,
                error_message TEXT,
                executed_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_time
             ON automation_execution_log(executed_at DESC)",
            [],
        )?;

        Ok(())
    }
}
