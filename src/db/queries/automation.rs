use crate::models::db_enum::DbEnum;
use crate::models::{
    AutomationAction, AutomationCondition, AutomationExecutionLog, AutomationRule,
    ComparisonOperator, LogicalOperator, SensorField, SwitchAction,
};
use rusqlite::{Result, params};
use serde_json;
use tracing::{debug, error, info};

use super::super::connection::{Database, MutexExt, Transaction};
use super::utils::{invalid_column_error, timestamp_to_datetime};

impl Database {
    /// Insert a new automation rule with its conditions and actions
    pub fn insert_automation_rule(&self, rule: &AutomationRule) -> Result<()> {
        debug!(
            rule_id = %rule.id,
            name = %rule.name,
            "Inserting automation rule"
        );

        let conn = self.conn.lock_or_recover();
        let start = std::time::Instant::now();

        // Begin transaction (auto-rollback on error or panic)
        let tx = Transaction::begin(&conn)?;

        // Insert the rule
        let active_days_json = match &rule.time_window.active_days {
            Some(days) => Some(
                serde_json::to_string(days)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            ),
            None => None,
        };

        conn.execute(
            "INSERT INTO automation_rules (id, name, description, enabled, condition_operator, created_at, updated_at, last_triggered_at, trigger_count, time_window_enabled, time_window_start, time_window_end, active_days)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                rule.id,
                rule.name,
                rule.description,
                if rule.enabled { 1 } else { 0 },
                rule.condition_operator.to_db_string(),
                rule.created_at.timestamp(),
                rule.updated_at.timestamp(),
                rule.last_triggered_at.map(|dt| dt.timestamp()),
                rule.trigger_count,
                if rule.time_window.enabled { 1 } else { 0 },
                rule.time_window.start_time.clone(),
                rule.time_window.end_time.clone(),
                active_days_json
            ],
        )?;

        // Insert conditions
        for condition in &rule.conditions {
            conn.execute(
                "INSERT INTO automation_conditions (id, rule_id, device_id, field, operator, value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    condition.id,
                    rule.id,
                    condition.device_id,
                    condition.field.to_db_string(),
                    condition.operator.to_db_string(),
                    condition.value
                ],
            )?;
        }

        // Insert actions
        for action in &rule.actions {
            conn.execute(
                "INSERT INTO automation_actions (id, rule_id, device_id, action)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    action.id,
                    rule.id,
                    action.device_id,
                    action.action.to_db_string()
                ],
            )?;
        }

        // Commit transaction
        tx.commit()?;

        let elapsed = start.elapsed();
        info!(
            rule_id = %rule.id,
            duration_ms = elapsed.as_millis(),
            "Successfully inserted automation rule"
        );

        Ok(())
    }

    /// Get all automation rules with their conditions and actions
    pub fn get_all_automation_rules(&self) -> Result<Vec<AutomationRule>> {
        debug!("Querying all automation rules");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();

        // Get all rules
        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, condition_operator, created_at, updated_at, last_triggered_at, trigger_count, time_window_enabled, time_window_start, time_window_end, active_days
             FROM automation_rules
             ORDER BY name",
        )?;

        let rule_rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let condition_operator_str: String = row.get(4)?;

            Ok((
                id,
                row.get::<_, String>(1)?,         // name
                row.get::<_, Option<String>>(2)?, // description
                row.get::<_, i32>(3)? == 1,       // enabled
                condition_operator_str,
                row.get::<_, i64>(5)?,         // created_at
                row.get::<_, i64>(6)?,         // updated_at
                row.get::<_, Option<i64>>(7)?, // last_triggered_at
                row.get::<_, i64>(8)?,         // trigger_count
                row.get::<_, Option<i32>>(9)?, // time_window_enabled
                row.get::<_, Option<String>>(10)?, // time_window_start
                row.get::<_, Option<String>>(11)?, // time_window_end
                row.get::<_, Option<String>>(12)?, // active_days
            ))
        })?;

        let mut rules = Vec::new();

        for rule_row_result in rule_rows {
            let (
                id,
                name,
                description,
                enabled,
                condition_operator_str,
                created_at,
                updated_at,
                last_triggered_at,
                trigger_count,
                time_window_enabled,
                time_window_start,
                time_window_end,
                active_days_json,
            ) = rule_row_result?;

            let condition_operator = LogicalOperator::from_db_string(&condition_operator_str)
                .ok_or_else(|| invalid_column_error(4, "condition_operator"))?;

            // Get conditions for this rule
            let mut cond_stmt = conn.prepare(
                "SELECT id, device_id, field, operator, value
                 FROM automation_conditions
                 WHERE rule_id = ?1",
            )?;

            let conditions: Vec<AutomationCondition> = cond_stmt
                .query_map(params![id], |row| {
                    let field_str: String = row.get(2)?;
                    let operator_str: String = row.get(3)?;

                    Ok(AutomationCondition {
                        id: row.get(0)?,
                        device_id: row.get(1)?,
                        field: SensorField::from_db_string(&field_str)
                            .ok_or_else(|| invalid_column_error(2, "field"))?,
                        operator: ComparisonOperator::from_db_string(&operator_str)
                            .ok_or_else(|| invalid_column_error(3, "operator"))?,
                        value: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>>>()?;

            // Get actions for this rule
            let mut action_stmt = conn.prepare(
                "SELECT id, device_id, action
                 FROM automation_actions
                 WHERE rule_id = ?1",
            )?;

            let actions: Vec<AutomationAction> = action_stmt
                .query_map(params![id], |row| {
                    let action_str: String = row.get(2)?;

                    Ok(AutomationAction {
                        id: row.get(0)?,
                        device_id: row.get(1)?,
                        action: SwitchAction::from_db_string(&action_str)
                            .ok_or_else(|| invalid_column_error(2, "action"))?,
                    })
                })?
                .collect::<Result<Vec<_>>>()?;

            let active_days = match active_days_json {
                Some(json) => Some(
                    serde_json::from_str::<Vec<u8>>(&json).map_err(|e| {
                        error!(
                            error = %e,
                            rule_id = %id,
                            "Failed to parse active_days JSON"
                        );
                        rusqlite::Error::FromSqlConversionFailure(
                            12,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                ),
                None => None,
            };

            rules.push(AutomationRule {
                id,
                name,
                description,
                enabled,
                condition_operator,
                conditions,
                actions,
                created_at: timestamp_to_datetime(created_at, "automation_rule.created_at"),
                updated_at: timestamp_to_datetime(updated_at, "automation_rule.updated_at"),
                last_triggered_at: last_triggered_at
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
                trigger_count,
                time_window: crate::models::TimeWindow {
                    enabled: time_window_enabled.unwrap_or(0) == 1,
                    start_time: time_window_start,
                    end_time: time_window_end,
                    active_days,
                },
            });
        }

        let elapsed = start.elapsed();
        info!(
            rule_count = rules.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all automation rules"
        );

        Ok(rules)
    }

    /// Get a single automation rule by ID
    pub fn get_automation_rule(&self, rule_id: &str) -> Result<Option<AutomationRule>> {
        debug!(rule_id = %rule_id, "Querying automation rule by ID");

        let all_rules = self.get_all_automation_rules()?;
        Ok(all_rules.into_iter().find(|r| r.id == rule_id))
    }

    /// Update an automation rule
    pub fn update_automation_rule(&self, rule: &AutomationRule) -> Result<()> {
        debug!(rule_id = %rule.id, "Updating automation rule");

        let conn = self.conn.lock_or_recover();
        let start = std::time::Instant::now();

        // Begin transaction (auto-rollback on error or panic)
        let tx = Transaction::begin(&conn)?;

        // Update the rule metadata
        let active_days_json = match &rule.time_window.active_days {
            Some(days) => Some(
                serde_json::to_string(days)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            ),
            None => None,
        };

        conn.execute(
            "UPDATE automation_rules
             SET name = ?1, description = ?2, enabled = ?3, condition_operator = ?4, updated_at = ?5, time_window_enabled = ?6, time_window_start = ?7, time_window_end = ?8, active_days = ?9
             WHERE id = ?10",
            params![
                rule.name,
                rule.description,
                if rule.enabled { 1 } else { 0 },
                rule.condition_operator.to_db_string(),
                rule.updated_at.timestamp(),
                if rule.time_window.enabled { 1 } else { 0 },
                rule.time_window.start_time.clone(),
                rule.time_window.end_time.clone(),
                active_days_json,
                rule.id
            ],
        )?;

        // Delete existing conditions and actions
        conn.execute(
            "DELETE FROM automation_conditions WHERE rule_id = ?1",
            params![rule.id],
        )?;
        conn.execute(
            "DELETE FROM automation_actions WHERE rule_id = ?1",
            params![rule.id],
        )?;

        // Insert new conditions
        for condition in &rule.conditions {
            conn.execute(
                "INSERT INTO automation_conditions (id, rule_id, device_id, field, operator, value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    condition.id,
                    rule.id,
                    condition.device_id,
                    condition.field.to_db_string(),
                    condition.operator.to_db_string(),
                    condition.value
                ],
            )?;
        }

        // Insert new actions
        for action in &rule.actions {
            conn.execute(
                "INSERT INTO automation_actions (id, rule_id, device_id, action)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    action.id,
                    rule.id,
                    action.device_id,
                    action.action.to_db_string()
                ],
            )?;
        }

        // Commit transaction
        tx.commit()?;

        let elapsed = start.elapsed();
        info!(
            rule_id = %rule.id,
            duration_ms = elapsed.as_millis(),
            "Successfully updated automation rule"
        );

        Ok(())
    }

    /// Delete an automation rule (cascades to conditions and actions)
    pub fn delete_automation_rule(&self, rule_id: &str) -> Result<()> {
        debug!(rule_id = %rule_id, "Deleting automation rule");

        let start = std::time::Instant::now();
        let result = self.conn.lock_or_recover().execute(
            "DELETE FROM automation_rules WHERE id = ?1",
            params![rule_id],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    rule_id = %rule_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully deleted automation rule"
                );
                Ok(())
            }
            Err(e) => {
                error!(error = %e, rule_id = %rule_id, "Failed to delete automation rule");
                Err(e)
            }
        }
    }

    /// Update rule's last triggered timestamp and increment trigger count
    pub fn record_rule_trigger(&self, rule_id: &str) -> Result<()> {
        debug!(rule_id = %rule_id, "Recording rule trigger");

        let now = chrono::Utc::now().timestamp();
        let result = self.conn.lock_or_recover().execute(
            "UPDATE automation_rules
             SET last_triggered_at = ?1, trigger_count = trigger_count + 1
             WHERE id = ?2",
            params![now, rule_id],
        );

        match result {
            Ok(_) => {
                debug!(rule_id = %rule_id, "Recorded rule trigger");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, rule_id = %rule_id, "Failed to record rule trigger");
                Err(e)
            }
        }
    }

    /// Insert an execution log entry
    pub fn insert_execution_log(&self, log: &AutomationExecutionLog) -> Result<()> {
        debug!(
            log_id = %log.id,
            rule_id = %log.rule_id,
            success = %log.success,
            "Inserting execution log"
        );

        let result = self.conn.lock_or_recover().execute(
            "INSERT INTO automation_execution_log (id, rule_id, rule_name, success, error_message, executed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                log.id,
                log.rule_id,
                log.rule_name,
                if log.success { 1 } else { 0 },
                log.error_message,
                log.executed_at.timestamp()
            ],
        );

        match result {
            Ok(_) => {
                debug!(log_id = %log.id, "Inserted execution log");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, log_id = %log.id, "Failed to insert execution log");
                Err(e)
            }
        }
    }

    /// Get recent execution logs (limited to last N entries)
    pub fn get_execution_logs(&self, limit: i64) -> Result<Vec<AutomationExecutionLog>> {
        debug!(limit = %limit, "Querying execution logs");

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, rule_id, rule_name, success, error_message, executed_at
             FROM automation_execution_log
             ORDER BY executed_at DESC
             LIMIT ?1",
        )?;

        let logs = stmt
            .query_map(params![limit], |row| {
                Ok(AutomationExecutionLog {
                    id: row.get(0)?,
                    rule_id: row.get(1)?,
                    rule_name: row.get(2)?,
                    success: row.get::<_, i32>(3)? == 1,
                    error_message: row.get(4)?,
                    executed_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                        .unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        info!(log_count = logs.len(), "Retrieved execution logs");
        Ok(logs)
    }
}
