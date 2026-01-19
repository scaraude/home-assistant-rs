use std::collections::HashMap;
use std::sync::Arc;

use crate::db::Database;
use crate::events::{SystemEvent, bus::EventBus};
use crate::models::{
    AutomationAction, AutomationCondition, AutomationRule, SensorField, SensorReading,
    SwitchAction, SwitchCommandMessage, SwitchState,
};
use crate::mqtt::MqttClient;
use crate::state::{DeviceStateStore, SwitchStateStore};
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};

/// Service responsible for evaluating automation rules and executing actions in response
/// to incoming sensor readings from the event bus.
pub struct AutomationService {
    db: Arc<Database>,
    mqtt_client: MqttClient,
    device_state: DeviceStateStore,
    switch_state: SwitchStateStore,
    event_bus: EventBus,
    event_rx: broadcast::Receiver<SystemEvent>,
    last_triggered: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    debounce_seconds: i64,
}

impl AutomationService {
    pub fn new(
        db: Arc<Database>,
        mqtt_client: MqttClient,
        device_state: DeviceStateStore,
        switch_state: SwitchStateStore,
        event_bus: EventBus,
        event_rx: broadcast::Receiver<SystemEvent>,
    ) -> Self {
        Self {
            db,
            mqtt_client,
            device_state,
            switch_state,
            event_bus,
            event_rx,
            last_triggered: Arc::new(RwLock::new(HashMap::new())),
            debounce_seconds: 60,
        }
    }

    /// Run the automation event loop until the bus closes.
    pub async fn run(mut self) {
        info!("AutomationService started");

        loop {
            match self.event_rx.recv().await {
                Ok(event) => self.process_event(event).await,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "AutomationService lagged behind the event bus"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Event bus closed; AutomationService exiting");
                    break;
                }
            }
        }
    }

    async fn process_event(&self, event: SystemEvent) {
        if let SystemEvent::SensorReading { reading, .. } = event {
            self.handle_sensor_reading(reading).await;
        }
    }

    async fn handle_sensor_reading(&self, reading: SensorReading) {
        debug!(
            device_id = %reading.device_id(),
            "AutomationService received sensor reading"
        );

        let triggered_rules = self.evaluate_reading_internal(&reading).await;
        if triggered_rules.is_empty() {
            return;
        }

        for rule in triggered_rules {
            let trigger_timestamp = Utc::now();
            let trigger_event = SystemEvent::AutomationTriggered {
                rule_id: rule.id.clone(),
                actions: rule.actions.clone(),
                timestamp: trigger_timestamp,
            };
            self.publish_event(trigger_event);

            match self.execute_actions(&rule).await {
                Ok(_) => {
                    let event = SystemEvent::AutomationExecuted {
                        rule_id: rule.id.clone(),
                        success: true,
                        error: None,
                        timestamp: Utc::now(),
                    };
                    self.publish_event(event);
                }
                Err(err_msg) => {
                    let event = SystemEvent::AutomationExecuted {
                        rule_id: rule.id.clone(),
                        success: false,
                        error: Some(err_msg.clone()),
                        timestamp: Utc::now(),
                    };
                    self.publish_event(event);
                    error!(
                        rule_id = %rule.id,
                        rule_name = %rule.name,
                        error = %err_msg,
                        "Automation rule execution failed"
                    );
                }
            }
        }
    }

    async fn evaluate_reading_internal(&self, reading: &SensorReading) -> Vec<AutomationRule> {
        let rules = match self.db.get_all_automation_rules() {
            Ok(rules) => rules.into_iter().filter(|r| r.enabled).collect::<Vec<_>>(),
            Err(e) => {
                error!(error = %e, "Failed to fetch automation rules");
                return Vec::new();
            }
        };

        if rules.is_empty() {
            debug!("No enabled automation rules to evaluate");
            return Vec::new();
        }

        let mut triggered = Vec::new();
        for rule in rules {
            if !Self::rule_targets_device(&rule, reading.device_id()) {
                continue;
            }

            if self.should_debounce(&rule.id).await {
                continue;
            }

            if !self.is_within_time_window(&rule) {
                debug!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Automation rule skipped due to time window"
                );
                continue;
            }

            if self.evaluate_conditions(&rule).await {
                info!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Automation rule conditions met"
                );
                self.mark_triggered(&rule.id).await;
                triggered.push(rule);
            } else {
                debug!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Automation rule conditions not met"
                );
            }
        }

        triggered
    }

    async fn should_debounce(&self, rule_id: &str) -> bool {
        let guard = self.last_triggered.read().await;
        if let Some(last_time) = guard.get(rule_id) {
            let elapsed = Utc::now().signed_duration_since(*last_time);
            if elapsed.num_seconds() < self.debounce_seconds {
                debug!(
                    rule_id = %rule_id,
                    seconds_since_last = elapsed.num_seconds(),
                    "Automation rule debounced"
                );
                return true;
            }
        }
        false
    }

    async fn mark_triggered(&self, rule_id: &str) {
        let mut guard = self.last_triggered.write().await;
        guard.insert(rule_id.to_string(), Utc::now());
    }

    fn is_within_time_window(&self, rule: &AutomationRule) -> bool {
        Self::is_within_time_window_at(rule, Local::now())
    }

    fn is_within_time_window_at(rule: &AutomationRule, now: DateTime<Local>) -> bool {
        let window = &rule.time_window;
        if !window.enabled {
            return true;
        }

        let start_minutes = match window.start_time.as_deref().and_then(parse_time_minutes) {
            Some(minutes) => minutes,
            None => {
                error!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Time window enabled but start_time is missing or invalid"
                );
                return false;
            }
        };

        let end_minutes = match window.end_time.as_deref().and_then(parse_time_minutes) {
            Some(minutes) => minutes,
            None => {
                error!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Time window enabled but end_time is missing or invalid"
                );
                return false;
            }
        };

        if let Some(active_days) = &window.active_days {
            if active_days.is_empty() {
                return false;
            }
            let today = now.weekday().num_days_from_sunday() as u8;
            if !active_days.contains(&today) {
                return false;
            }
        }

        let now_minutes = now.hour() * 60 + now.minute();
        if start_minutes <= end_minutes {
            now_minutes >= start_minutes && now_minutes <= end_minutes
        } else {
            now_minutes >= start_minutes || now_minutes <= end_minutes
        }
    }

    fn rule_targets_device(rule: &AutomationRule, device_id: &str) -> bool {
        rule.conditions
            .iter()
            .any(|condition| condition.device_id == device_id)
    }

    async fn evaluate_conditions(&self, rule: &AutomationRule) -> bool {
        if rule.conditions.is_empty() {
            warn!(rule_id = %rule.id, "Automation rule has no conditions");
            return false;
        }

        let mut results = Vec::with_capacity(rule.conditions.len());
        for condition in &rule.conditions {
            let result = self.evaluate_condition(condition).await;
            results.push(result);

            debug!(
                rule_id = %rule.id,
                device_id = %condition.device_id,
                field = ?condition.field,
                operator = ?condition.operator,
                value = %condition.value,
                result = %result,
                "Evaluated automation condition"
            );
        }

        match rule.condition_operator {
            crate::models::LogicalOperator::And => results.iter().all(|&r| r),
            crate::models::LogicalOperator::Or => results.iter().any(|&r| r),
        }
    }

    async fn evaluate_condition(&self, condition: &AutomationCondition) -> bool {
        let field_value = match condition.field {
            SensorField::Temperature => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::TempHumidity { temperature, .. } = reading {
                        Some(temperature as f64)
                    } else {
                        None
                    }
                }),
            SensorField::Humidity => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::TempHumidity { humidity, .. } = reading {
                        Some(humidity as f64)
                    } else {
                        None
                    }
                }),
            SensorField::Presence => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::Presence { occupied, .. } = reading {
                        Some(if occupied { 1.0 } else { 0.0 })
                    } else {
                        None
                    }
                }),
            SensorField::Illumination => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::Presence { illumination, .. } = reading {
                        illumination.as_ref().map(|ill| match ill.as_str() {
                            "bright" => 1.0,
                            "dim" => 0.0,
                            _ => 0.0, // default to dim for unknown values
                        })
                    } else {
                        None
                    }
                }),
            SensorField::Battery => self
                .device_state
                .get_battery(&condition.device_id)
                .map(|b| b as f64),
            SensorField::LinkQuality => self
                .device_state
                .get_link_quality(&condition.device_id)
                .map(|lq| lq as f64),
        };

        let value = match field_value {
            Some(value) => value,
            None => {
                debug!(
                    device_id = %condition.device_id,
                    field = ?condition.field,
                    "Automation condition field value unavailable"
                );
                return false;
            }
        };

        condition.operator.evaluate(value, condition.value)
    }

    async fn execute_actions(&self, rule: &AutomationRule) -> Result<(), String> {
        let mut errors = Vec::new();

        for action in &rule.actions {
            if let Err(err) = self.execute_action(action).await {
                errors.push(format!("{}: {}", action.device_id, err));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    async fn execute_action(&self, action: &AutomationAction) -> Result<(), String> {
        debug!(
            device_id = %action.device_id,
            action = ?action.action,
            "Executing automation action"
        );

        let mqtt_topic = self
            .db
            .get_mqtt_topic_for_device(&action.device_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Device not found: {}", action.device_id))?;

        let desired_state = match action.action {
            SwitchAction::On => SwitchState::On,
            SwitchAction::Off => SwitchState::Off,
            SwitchAction::Toggle => {
                if let Some(current) = self.switch_state.get_state(&action.device_id) {
                    current.toggled()
                } else {
                    return Err(format!(
                        "Current state unknown for device {}",
                        action.device_id
                    ));
                }
            }
        };

        let payload = serde_json::to_string(&SwitchCommandMessage {
            state: desired_state,
        })
        .map_err(|e| format!("MQTT payload serialization error: {}", e))?;

        self.mqtt_client
            .publish_command(&mqtt_topic, &payload)
            .await
            .map_err(|e| format!("MQTT publish error: {}", e))?;

        info!(
            device_id = %action.device_id,
            action = ?action.action,
            "Automation action executed successfully"
        );

        Ok(())
    }

    fn publish_event(&self, event: SystemEvent) {
        if let Err(e) = self.event_bus.publish(event.clone()) {
            error!(
                error = %e,
                event_type = event.event_type(),
                "Failed to publish automation event"
            );
        }
    }
}

fn parse_time_minutes(value: &str) -> Option<u32> {
    let mut parts = value.split(':');
    let hours = parts.next()?;
    let minutes = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }

    Some(hours * 60 + minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AutomationRule, LogicalOperator, TimeWindow};
    use chrono::TimeZone;

    fn rule_with_window(window: TimeWindow) -> AutomationRule {
        AutomationRule {
            id: "rule1".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: window,
        }
    }

    #[test]
    fn time_window_disabled_allows() {
        let rule = rule_with_window(TimeWindow::default());
        let now = Local.with_ymd_and_hms(2024, 4, 5, 12, 0, 0).unwrap();
        assert!(AutomationService::is_within_time_window_at(&rule, now));
    }

    #[test]
    fn time_window_same_day_range() {
        let rule = rule_with_window(TimeWindow {
            enabled: true,
            start_time: Some("08:00".to_string()),
            end_time: Some("10:00".to_string()),
            active_days: None,
        });
        let in_window = Local.with_ymd_and_hms(2024, 4, 5, 9, 0, 0).unwrap();
        let out_window = Local.with_ymd_and_hms(2024, 4, 5, 11, 0, 0).unwrap();
        assert!(AutomationService::is_within_time_window_at(
            &rule, in_window
        ));
        assert!(!AutomationService::is_within_time_window_at(
            &rule, out_window
        ));
    }

    #[test]
    fn time_window_overnight_range() {
        let rule = rule_with_window(TimeWindow {
            enabled: true,
            start_time: Some("22:00".to_string()),
            end_time: Some("06:00".to_string()),
            active_days: None,
        });
        let late = Local.with_ymd_and_hms(2024, 4, 5, 23, 0, 0).unwrap();
        let early = Local.with_ymd_and_hms(2024, 4, 6, 5, 0, 0).unwrap();
        let midday = Local.with_ymd_and_hms(2024, 4, 5, 12, 0, 0).unwrap();
        assert!(AutomationService::is_within_time_window_at(&rule, late));
        assert!(AutomationService::is_within_time_window_at(&rule, early));
        assert!(!AutomationService::is_within_time_window_at(&rule, midday));
    }

    #[test]
    fn time_window_active_days() {
        let rule = rule_with_window(TimeWindow {
            enabled: true,
            start_time: Some("08:00".to_string()),
            end_time: Some("20:00".to_string()),
            active_days: Some(vec![1, 3, 5]),
        });
        let monday = Local.with_ymd_and_hms(2024, 4, 1, 12, 0, 0).unwrap();
        let tuesday = Local.with_ymd_and_hms(2024, 4, 2, 12, 0, 0).unwrap();
        assert!(AutomationService::is_within_time_window_at(&rule, monday));
        assert!(!AutomationService::is_within_time_window_at(&rule, tuesday));
    }
}
