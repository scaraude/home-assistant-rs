use super::connection::{Database, MutexExt};
use rusqlite::Result;
use tracing::{debug, error, info, warn};

impl Database {
    /// Initialize the database schema
    pub fn init_schema(&self) -> Result<()> {
        debug!("Acquiring database lock for schema initialization");
        let conn = self.conn.lock_or_recover();

        // Create devices table first (referenced by foreign keys)
        self.create_devices_table(&conn)?;

        // Create device_state table
        self.create_device_state_table(&conn)?;

        // Create switch_state table
        self.create_switch_state_table(&conn)?;

        // Create temperature_readings table
        self.create_temperature_readings_table(&conn)?;

        // Create presence_readings table
        self.create_presence_readings_table(&conn)?;

        // Create energy_readings table
        self.create_energy_readings_table(&conn)?;

        // Create automation tables
        self.create_automation_tables(&conn)?;

        // Create device_positions table for floor map
        self.create_device_positions_table(&conn)?;

        // Create floor_plan table for floor map SVG
        self.create_floor_plan_table(&conn)?;

        // Repair foreign keys if a previous migration renamed devices
        self.repair_device_foreign_keys(&conn)?;

        info!("Database schema initialization complete");
        Ok(())
    }

    /// Create switch_state table for historical switch state changes
    fn create_switch_state_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating switch_state table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS switch_state (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                state INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
            )",
            [],
        ) {
            Ok(_) => debug!("Switch state table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create switch_state table");
                return Err(e);
            }
        }

        debug!("Creating index idx_switch_state_latest if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_switch_state_latest
             ON switch_state(device_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_switch_state_latest created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create idx_switch_state_latest index");
                return Err(e);
            }
        }

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
                humidity REAL NOT NULL,
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

        Ok(())
    }

    /// Create presence_readings table and indexes
    fn create_presence_readings_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating presence_readings table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS presence_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                occupied INTEGER NOT NULL,
                illumination TEXT,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
            )",
            [],
        ) {
            Ok(_) => debug!("Presence readings table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create presence_readings table");
                return Err(e);
            }
        }

        // Index for efficient queries by device and time
        debug!("Creating index idx_presence_time if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_presence_time
             ON presence_readings(device_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_presence_time created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_presence_time");
                return Err(e);
            }
        }

        // Migration: Add illumination column if it doesn't exist
        debug!("Checking for illumination column in presence_readings table");
        let column_exists = conn
            .prepare("SELECT illumination FROM presence_readings LIMIT 1")
            .is_ok();

        if !column_exists {
            debug!("Adding illumination column to presence_readings table");
            match conn.execute(
                "ALTER TABLE presence_readings ADD COLUMN illumination TEXT",
                [],
            ) {
                Ok(_) => info!("Added illumination column to presence_readings table"),
                Err(e) => {
                    error!(error = %e, "Failed to add illumination column");
                    return Err(e);
                }
            }
        } else {
            debug!("Illumination column already exists");
        }

        // Migration: Ensure all sensors have capability metadata in devices table
        debug!("Checking for sensors without capability metadata");

        // Count devices missing capability info
        let missing_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM (
                    SELECT device_id FROM temperature_readings
                    UNION
                    SELECT device_id FROM presence_readings
                ) WHERE device_id NOT IN (SELECT id FROM devices)",
            [],
            |row| row.get(0),
        )?;

        if missing_count > 0 {
            warn!(
                missing_devices = missing_count,
                "Found sensors without capability metadata, inserting records"
            );

            // Insert missing temperature sensors
            let temp_inserted = conn.execute(
                "INSERT OR IGNORE INTO devices (id, name, mqtt_topic, capabilities, available_fields, power_source, added_at)
                 SELECT DISTINCT
                     device_id,
                     device_id,
                     'zigbee2mqtt/' || device_id,
                     '[{\"type\":\"sensor\",\"sensor_type\":\"temp_humidity\"}]',
                     '[]',
                     'plugged',
                     strftime('%s','now')
                 FROM temperature_readings
                 WHERE device_id NOT IN (SELECT id FROM devices)",
                [],
            )?;

            // Insert missing presence sensors
            let presence_inserted = conn.execute(
                "INSERT OR IGNORE INTO devices (id, name, mqtt_topic, capabilities, available_fields, power_source, added_at)
                 SELECT DISTINCT
                     device_id,
                     device_id,
                     'zigbee2mqtt/' || device_id,
                     '[{\"type\":\"sensor\",\"sensor_type\":\"presence\"}]',
                     '[]',
                     'plugged',
                     strftime('%s','now')
                 FROM presence_readings
                 WHERE device_id NOT IN (SELECT id FROM devices)",
                [],
            )?;

            info!(
                temp_sensors_inserted = temp_inserted,
                presence_sensors_inserted = presence_inserted,
                "Migration: Inserted capability metadata for all sensors"
            );
        } else {
            debug!("All sensors already have capability metadata");
        }

        Ok(())
    }

    /// Create energy_readings table and indexes
    fn create_energy_readings_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating energy_readings table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS energy_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                power REAL NOT NULL,
                energy REAL NOT NULL,
                produced_energy REAL NOT NULL,
                voltage REAL NOT NULL,
                current REAL NOT NULL,
                ac_frequency REAL NOT NULL,
                power_factor REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
            )",
            [],
        ) {
            Ok(_) => debug!("Energy readings table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create energy_readings table");
                return Err(e);
            }
        }

        // Index for efficient queries by device and time
        debug!("Creating index idx_energy_time if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_energy_time
             ON energy_readings(device_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_energy_time created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_energy_time");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Create device_state table for ephemeral device state (battery, link_quality, last_seen)
    fn create_device_state_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating device_state table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS device_state (
                device_id TEXT PRIMARY KEY,
                battery_level INTEGER,
                link_quality INTEGER,
                turbo_mode INTEGER,
                last_seen INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
            )",
            [],
        ) {
            Ok(_) => debug!("Device state table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create device_state table");
                return Err(e);
            }
        }

        self.ensure_device_state_columns(conn)?;

        // Index for efficient last_seen queries
        debug!("Creating index idx_device_state_last_seen if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_device_state_last_seen
             ON device_state(last_seen DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_device_state_last_seen created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_device_state_last_seen");
                return Err(e);
            }
        }

        Ok(())
    }

    fn ensure_device_state_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(device_state)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_turbo_mode = false;

        for column in columns {
            if column? == "turbo_mode" {
                has_turbo_mode = true;
                break;
            }
        }

        if !has_turbo_mode {
            info!("Adding turbo_mode column to device_state table");
            conn.execute("ALTER TABLE device_state ADD COLUMN turbo_mode INTEGER", [])?;
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
                ieee_addr TEXT,
                name TEXT NOT NULL,
                capabilities TEXT NOT NULL DEFAULT '[]',
                available_fields TEXT NOT NULL DEFAULT '[]',
                power_source TEXT NOT NULL,
                added_at INTEGER NOT NULL,
                color TEXT
            )",
            [],
        ) {
            Ok(_) => debug!("Devices table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create devices table");
                return Err(e);
            }
        }

        // Add identity columns if missing
        self.ensure_device_identity_columns(conn)?;

        // Add network topology columns if missing
        self.ensure_device_network_columns(conn)?;

        // Add capability columns if missing
        self.ensure_device_capability_columns(conn)?;

        // Add UI columns if missing
        self.ensure_device_ui_columns(conn)?;

        // Drop legacy columns after migration
        self.drop_legacy_device_columns(conn)?;

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

        // Create index on ieee_addr for fast lookups
        debug!("Creating index idx_ieee_addr if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ieee_addr ON devices(ieee_addr)",
            [],
        ) {
            Ok(_) => debug!("Index idx_ieee_addr created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_ieee_addr");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Ensure identity columns exist in devices table
    fn ensure_device_identity_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        self.add_column_if_missing(conn, "devices", "ieee_addr", "TEXT")?;
        self.backfill_device_ieee_addr(conn)?;
        Ok(())
    }

    /// Ensure network topology columns exist in devices table
    fn ensure_device_network_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        self.add_column_if_missing(conn, "devices", "is_bridge", "INTEGER DEFAULT 0")?;
        self.add_column_if_missing(conn, "devices", "parent_device_id", "TEXT")?;
        Ok(())
    }

    fn ensure_device_capability_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        self.add_column_if_missing(
            conn,
            "devices",
            "capabilities",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        self.add_column_if_missing(
            conn,
            "devices",
            "available_fields",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        self.migrate_legacy_device_capabilities(conn)?;
        Ok(())
    }

    /// Ensure UI-related columns exist in devices table
    fn ensure_device_ui_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        self.add_column_if_missing(conn, "devices", "color", "TEXT")?;
        Ok(())
    }

    fn migrate_legacy_device_capabilities(&self, conn: &rusqlite::Connection) -> Result<()> {
        use crate::models::db_enum::DbEnum;
        use crate::models::{CommanderType, Device, DeviceCapability, SensorType};
        use rusqlite::params;

        let has_capability_type = self.column_exists(conn, "devices", "capability_type")?;
        let has_capability_subtype = self.column_exists(conn, "devices", "capability_subtype")?;
        if !(has_capability_type && has_capability_subtype) {
            return Ok(());
        }

        let mut stmt = conn.prepare(
            "SELECT id, capability_type, capability_subtype, is_bridge, capabilities
             FROM devices",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let device_id: String = row.get(0)?;
            let cap_type: Option<String> = row.get(1)?;
            let cap_subtype: Option<String> = row.get(2)?;
            let is_bridge: i32 = row.get(3).unwrap_or(0);
            let current_caps: Option<String> = row.get(4)?;

            let existing_caps = current_caps
                .as_deref()
                .and_then(Device::capabilities_from_db)
                .unwrap_or_default();
            if !existing_caps.is_empty() {
                continue;
            }

            let mut capabilities = Vec::new();
            match (cap_type.as_deref(), cap_subtype.as_deref()) {
                (Some("sensor"), Some(subtype)) => {
                    if let Some(sensor_type) = SensorType::from_db_string(subtype) {
                        capabilities.push(DeviceCapability::Sensor { sensor_type });
                    }
                }
                (Some("commander"), Some(subtype)) => {
                    if let Some(commander_type) = CommanderType::from_db_string(subtype) {
                        capabilities.push(DeviceCapability::Commander { commander_type });
                    }
                }
                (Some("coordinator"), _) => {
                    capabilities.push(DeviceCapability::Coordinator);
                }
                _ => {}
            }

            if is_bridge != 0 {
                capabilities.push(DeviceCapability::Router { turbo_mode: false });
            }

            if capabilities.is_empty() {
                continue;
            }

            let caps_json = serde_json::to_string(&capabilities)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            conn.execute(
                "UPDATE devices SET capabilities = ?1 WHERE id = ?2",
                params![caps_json, device_id],
            )?;
        }

        Ok(())
    }

    fn drop_legacy_device_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        let has_capability_type = self.column_exists(conn, "devices", "capability_type")?;
        let has_capability_subtype = self.column_exists(conn, "devices", "capability_subtype")?;
        if !(has_capability_type && has_capability_subtype) {
            return Ok(());
        }

        info!("Dropping legacy device capability columns");
        conn.execute_batch(
            "PRAGMA foreign_keys=OFF;
             BEGIN;
             CREATE TABLE devices_new (
                 id TEXT PRIMARY KEY,
                 mqtt_topic TEXT NOT NULL UNIQUE,
                 ieee_addr TEXT,
                 name TEXT NOT NULL,
                 capabilities TEXT NOT NULL DEFAULT '[]',
                 available_fields TEXT NOT NULL DEFAULT '[]',
                 power_source TEXT NOT NULL,
                 added_at INTEGER NOT NULL,
                 is_bridge INTEGER DEFAULT 0,
                 parent_device_id TEXT,
                 color TEXT
             );
             INSERT INTO devices_new (id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id, color)
             SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id, color
             FROM devices;
             DROP TABLE devices;
             ALTER TABLE devices_new RENAME TO devices;
             CREATE INDEX IF NOT EXISTS idx_mqtt_topic ON devices(mqtt_topic);
             CREATE INDEX IF NOT EXISTS idx_ieee_addr ON devices(ieee_addr);
             COMMIT;
             PRAGMA foreign_keys=ON;",
        )?;
        Ok(())
    }

    fn repair_device_foreign_keys(&self, conn: &rusqlite::Connection) -> Result<()> {
        let tables = [
            "device_state",
            "switch_state",
            "temperature_readings",
            "presence_readings",
            "energy_readings",
        ];

        for table in tables {
            if self.table_fk_targets_legacy_devices(conn, table)? {
                info!(table = %table, "Rebuilding table to repair foreign keys");
                self.rebuild_table_with_device_fk(conn, table)?;
            }
        }

        Ok(())
    }

    fn table_fk_targets_legacy_devices(
        &self,
        conn: &rusqlite::Connection,
        table: &str,
    ) -> Result<bool> {
        let pragma = format!("PRAGMA foreign_key_list({})", table);
        let mut stmt = conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let target_table: String = row.get(2)?;
            let from_col: String = row.get(3)?;
            if from_col == "device_id" && target_table != "devices" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn rebuild_table_with_device_fk(&self, conn: &rusqlite::Connection, table: &str) -> Result<()> {
        let (create_sql, insert_sql, index_sql) = match table {
            "device_state" => (
                "CREATE TABLE device_state_rebuild (
                    device_id TEXT PRIMARY KEY,
                    battery_level INTEGER,
                    link_quality INTEGER,
                    turbo_mode INTEGER,
                    last_seen INTEGER NOT NULL,
                    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
                )",
                "INSERT INTO device_state_rebuild (device_id, battery_level, link_quality, turbo_mode, last_seen)
                 SELECT device_id, battery_level, link_quality, turbo_mode, last_seen FROM device_state",
                "CREATE INDEX IF NOT EXISTS idx_device_state_last_seen
                 ON device_state(last_seen DESC)",
            ),
            "switch_state" => (
                "CREATE TABLE switch_state_rebuild (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id TEXT NOT NULL,
                    state INTEGER NOT NULL,
                    timestamp INTEGER NOT NULL,
                    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
                )",
                "INSERT INTO switch_state_rebuild (id, device_id, state, timestamp)
                 SELECT id, device_id, state, timestamp FROM switch_state",
                "CREATE INDEX IF NOT EXISTS idx_switch_state_latest
                 ON switch_state(device_id, timestamp DESC)",
            ),
            "temperature_readings" => (
                "CREATE TABLE temperature_readings_rebuild (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id TEXT NOT NULL,
                    temperature REAL NOT NULL,
                    humidity REAL NOT NULL,
                    timestamp INTEGER NOT NULL,
                    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
                )",
                "INSERT INTO temperature_readings_rebuild (id, device_id, temperature, humidity, timestamp)
                 SELECT id, device_id, temperature, humidity, timestamp FROM temperature_readings",
                "CREATE INDEX IF NOT EXISTS idx_sensor_time
                 ON temperature_readings(device_id, timestamp DESC)",
            ),
            "presence_readings" => (
                "CREATE TABLE presence_readings_rebuild (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id TEXT NOT NULL,
                    occupied INTEGER NOT NULL,
                    illumination TEXT,
                    timestamp INTEGER NOT NULL,
                    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
                )",
                "INSERT INTO presence_readings_rebuild (id, device_id, occupied, illumination, timestamp)
                 SELECT id, device_id, occupied, illumination, timestamp FROM presence_readings",
                "CREATE INDEX IF NOT EXISTS idx_presence_time
                 ON presence_readings(device_id, timestamp DESC)",
            ),
            "energy_readings" => (
                "CREATE TABLE energy_readings_rebuild (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id TEXT NOT NULL,
                    power REAL NOT NULL,
                    energy REAL NOT NULL,
                    produced_energy REAL NOT NULL,
                    voltage REAL NOT NULL,
                    current REAL NOT NULL,
                    ac_frequency REAL NOT NULL,
                    power_factor REAL NOT NULL,
                    timestamp INTEGER NOT NULL,
                    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
                )",
                "INSERT INTO energy_readings_rebuild (id, device_id, power, energy, produced_energy, voltage, current, ac_frequency, power_factor, timestamp)
                 SELECT id, device_id, power, energy, produced_energy, voltage, current, ac_frequency, power_factor, timestamp FROM energy_readings",
                "CREATE INDEX IF NOT EXISTS idx_energy_time
                 ON energy_readings(device_id, timestamp DESC)",
            ),
            _ => return Ok(()),
        };

        let rebuild_table = format!("{}_rebuild", table);
        let drop_sql = format!("DROP TABLE {}", table);
        let rename_sql = format!("ALTER TABLE {} RENAME TO {}", rebuild_table, table);

        conn.execute_batch(&format!(
            "PRAGMA foreign_keys=OFF;
                 BEGIN;
                 {};
                 {};
                 {};
                 {};
                 {};
                 COMMIT;
                 PRAGMA foreign_keys=ON;",
            create_sql, insert_sql, drop_sql, rename_sql, index_sql
        ))?;

        Ok(())
    }

    fn backfill_device_ieee_addr(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Backfilling missing device IEEE addresses from mqtt_topic");
        conn.execute(
            "UPDATE devices SET ieee_addr = mqtt_topic WHERE ieee_addr IS NULL OR ieee_addr = ''",
            [],
        )?;
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
                trigger_count INTEGER DEFAULT 0,
                time_window_enabled INTEGER DEFAULT 0,
                time_window_start TEXT,
                time_window_end TEXT,
                active_days TEXT
            )",
            [],
        )?;
        self.ensure_automation_rule_columns(conn)?;

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

    fn ensure_automation_rule_columns(&self, conn: &rusqlite::Connection) -> Result<()> {
        self.add_column_if_missing(
            conn,
            "automation_rules",
            "time_window_enabled",
            "INTEGER DEFAULT 0",
        )?;
        self.add_column_if_missing(conn, "automation_rules", "time_window_start", "TEXT")?;
        self.add_column_if_missing(conn, "automation_rules", "time_window_end", "TEXT")?;
        self.add_column_if_missing(conn, "automation_rules", "active_days", "TEXT")?;
        Ok(())
    }

    /// Create device_positions table for floor map view
    fn create_device_positions_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating device_positions table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS device_positions (
                device_id TEXT PRIMARY KEY,
                x REAL NOT NULL,
                y REAL NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
            )",
            [],
        ) {
            Ok(_) => debug!("Device positions table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create device_positions table");
                return Err(e);
            }
        }

        // Index on device_id for fast lookups (primary key already provides this,
        // but we add an index on updated_at for potential ordering queries)
        debug!("Creating index idx_device_positions_updated if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_device_positions_updated
             ON device_positions(updated_at DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_device_positions_updated created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create idx_device_positions_updated index");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Create floor_plan singleton table for floor map SVG storage
    fn create_floor_plan_table(&self, conn: &rusqlite::Connection) -> Result<()> {
        debug!("Creating floor_plan table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS floor_plan (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                svg_content TEXT NOT NULL,
                uploaded_at INTEGER NOT NULL
            )",
            [],
        ) {
            Ok(_) => debug!("Floor plan table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create floor_plan table");
                return Err(e);
            }
        }

        Ok(())
    }

    fn add_column_if_missing(
        &self,
        conn: &rusqlite::Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<()> {
        if self.column_exists(conn, table, column)? {
            return Ok(());
        }

        debug!(table = %table, column = %column, "Adding missing column");
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
        conn.execute(&sql, [])?;
        Ok(())
    }

    fn column_exists(
        &self,
        conn: &rusqlite::Connection,
        table: &str,
        column: &str,
    ) -> Result<bool> {
        let pragma = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
