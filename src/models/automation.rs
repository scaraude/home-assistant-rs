use crate::impl_db_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Comparison operator for condition evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    /// Equal to
    Equal,
    /// Not equal to
    NotEqual,
    /// Greater than
    GreaterThan,
    /// Greater than or equal to
    GreaterThanOrEqual,
    /// Less than
    LessThan,
    /// Less than or equal to
    LessThanOrEqual,
}

// Implement database string conversion using the DbEnum trait
impl_db_enum!(ComparisonOperator {
    Equal => "eq",
    NotEqual => "neq",
    GreaterThan => "gt",
    GreaterThanOrEqual => "gte",
    LessThan => "lt",
    LessThanOrEqual => "lte"
});

impl ComparisonOperator {
    /// Evaluate the comparison
    pub fn evaluate(&self, left: f64, right: f64) -> bool {
        match self {
            ComparisonOperator::Equal => (left - right).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (left - right).abs() >= f64::EPSILON,
            ComparisonOperator::GreaterThan => left > right,
            ComparisonOperator::GreaterThanOrEqual => left >= right,
            ComparisonOperator::LessThan => left < right,
            ComparisonOperator::LessThanOrEqual => left <= right,
        }
    }
}

/// Type of sensor field to monitor in a condition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SensorField {
    Temperature,
    Humidity,
    Battery,
    LinkQuality,
    Presence,
    Illumination,
}

// Implement database string conversion using the DbEnum trait
impl_db_enum!(SensorField {
    Temperature => "temperature",
    Humidity => "humidity",
    Battery => "battery",
    LinkQuality => "link_quality",
    Presence => "presence",
    Illumination => "illumination"
});

/// A single condition in an automation rule
/// Example: "temperature > 25" or "humidity < 40"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationCondition {
    /// Unique identifier for the condition
    pub id: String,

    /// The sensor device ID to monitor
    pub device_id: String,

    /// Which sensor field to check
    pub field: SensorField,

    /// Comparison operator
    pub operator: ComparisonOperator,

    /// Constant value to compare against (used when no variable target is set)
    pub value: f64,

    /// Optional "variable" target: another device to read the comparison value
    /// from. When set together with `target_field`, the condition compares
    /// `device_id.field` against `target_device_id.target_field` instead of the
    /// constant `value`. (Issue #7 — variables in rules.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_device_id: Option<String>,

    /// Which field to read on `target_device_id` (see `target_device_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_field: Option<SensorField>,
}

impl AutomationCondition {
    pub fn new(
        device_id: String,
        field: SensorField,
        operator: ComparisonOperator,
        value: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_id,
            field,
            operator,
            value,
            target_device_id: None,
            target_field: None,
        }
    }

    /// The variable target (device + field) if this condition compares against
    /// another sensor rather than a constant.
    pub fn variable_target(&self) -> Option<(&str, &SensorField)> {
        match (&self.target_device_id, &self.target_field) {
            (Some(device_id), Some(field)) => Some((device_id.as_str(), field)),
            _ => None,
        }
    }
}

impl From<CreateConditionRequest> for AutomationCondition {
    fn from(c: CreateConditionRequest) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: c.device_id,
            field: c.field,
            operator: c.operator,
            value: c.value,
            target_device_id: c.target_device_id,
            target_field: c.target_field,
        }
    }
}

/// Logical operator for combining multiple conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOperator {
    /// All conditions must be true
    And,
    /// At least one condition must be true
    Or,
}

// Implement database string conversion using the DbEnum trait
impl_db_enum!(LogicalOperator {
    And => "and",
    Or => "or"
});

/// Action to perform when conditions are met
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SwitchAction {
    /// Turn switch on
    On,
    /// Turn switch off
    Off,
    /// Toggle current state
    Toggle,
}

// Implement database string conversion using the DbEnum trait
impl_db_enum!(SwitchAction {
    On => "on",
    Off => "off",
    Toggle => "toggle"
});

/// An action to execute when rule conditions are met
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationAction {
    /// Unique identifier for the action
    pub id: String,

    /// The switch device to control
    pub device_id: String,

    /// Action to perform
    pub action: SwitchAction,
}

impl AutomationAction {
    pub fn new(device_id: String, action: SwitchAction) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_id,
            action,
        }
    }
}

/// Complete automation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    /// Unique identifier for the rule
    pub id: String,

    /// Human-readable name for the rule
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Whether the rule is currently active
    pub enabled: bool,

    /// How to combine multiple conditions (AND/OR)
    pub condition_operator: LogicalOperator,

    /// List of conditions that trigger this rule
    pub conditions: Vec<AutomationCondition>,

    /// List of actions to execute when conditions are met
    pub actions: Vec<AutomationAction>,

    /// When the rule was created
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,

    /// When the rule was last modified
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,

    /// When the rule was last triggered (if ever)
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_triggered_at: Option<DateTime<Utc>>,

    /// Count of how many times the rule has been triggered
    pub trigger_count: i64,

    /// Optional time window constraints for rule evaluation
    pub time_window: TimeWindow,
}

impl AutomationRule {
    pub fn new(
        name: String,
        description: Option<String>,
        condition_operator: LogicalOperator,
        conditions: Vec<AutomationCondition>,
        actions: Vec<AutomationAction>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            enabled: true,
            condition_operator,
            conditions,
            actions,
            created_at: now,
            updated_at: now,
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeWindow {
    pub enabled: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub active_days: Option<Vec<u8>>,
}

/// Request payload for creating a new automation rule
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAutomationRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub condition_operator: LogicalOperator,
    pub conditions: Vec<CreateConditionRequest>,
    pub actions: Vec<CreateActionRequest>,
    pub time_window: Option<TimeWindowRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateConditionRequest {
    pub device_id: String,
    pub field: SensorField,
    pub operator: ComparisonOperator,
    /// Constant comparison value. Ignored (and may be omitted) when a variable
    /// target is provided.
    #[serde(default)]
    pub value: f64,
    /// Optional variable target device (see `AutomationCondition`).
    #[serde(default)]
    pub target_device_id: Option<String>,
    /// Optional variable target field.
    #[serde(default)]
    pub target_field: Option<SensorField>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateActionRequest {
    pub device_id: String,
    pub action: SwitchAction,
}

/// Request payload for updating an automation rule
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateAutomationRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub condition_operator: Option<LogicalOperator>,
    pub conditions: Option<Vec<CreateConditionRequest>>,
    pub actions: Option<Vec<CreateActionRequest>>,
    pub time_window: Option<TimeWindowRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TimeWindowRequest {
    pub enabled: Option<bool>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub active_days: Option<Vec<u8>>,
}

/// Execution log entry for an automation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationExecutionLog {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub success: bool,
    pub error_message: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub executed_at: DateTime<Utc>,
}

impl AutomationExecutionLog {
    pub fn new(
        rule_id: String,
        rule_name: String,
        success: bool,
        error_message: Option<String>,
        executed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id,
            rule_name,
            success,
            error_message,
            executed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::db_enum::DbEnum;

    #[test]
    fn test_comparison_operator_conversions() {
        let operators = vec![
            (ComparisonOperator::Equal, "eq"),
            (ComparisonOperator::NotEqual, "neq"),
            (ComparisonOperator::GreaterThan, "gt"),
            (ComparisonOperator::GreaterThanOrEqual, "gte"),
            (ComparisonOperator::LessThan, "lt"),
            (ComparisonOperator::LessThanOrEqual, "lte"),
        ];

        for (op, expected_str) in operators {
            assert_eq!(op.to_db_string(), expected_str);
            assert_eq!(ComparisonOperator::from_db_string(expected_str), Some(op));
        }

        assert_eq!(ComparisonOperator::from_db_string("invalid"), None);
    }

    #[test]
    fn test_comparison_operator_evaluate() {
        assert!(ComparisonOperator::Equal.evaluate(5.0, 5.0));
        assert!(!ComparisonOperator::Equal.evaluate(5.0, 6.0));

        assert!(ComparisonOperator::NotEqual.evaluate(5.0, 6.0));
        assert!(!ComparisonOperator::NotEqual.evaluate(5.0, 5.0));

        assert!(ComparisonOperator::GreaterThan.evaluate(6.0, 5.0));
        assert!(!ComparisonOperator::GreaterThan.evaluate(5.0, 5.0));

        assert!(ComparisonOperator::GreaterThanOrEqual.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::GreaterThanOrEqual.evaluate(6.0, 5.0));

        assert!(ComparisonOperator::LessThan.evaluate(5.0, 6.0));
        assert!(!ComparisonOperator::LessThan.evaluate(5.0, 5.0));

        assert!(ComparisonOperator::LessThanOrEqual.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::LessThanOrEqual.evaluate(4.0, 5.0));
    }

    #[test]
    fn test_sensor_field_conversions() {
        let fields = vec![
            (SensorField::Temperature, "temperature"),
            (SensorField::Humidity, "humidity"),
            (SensorField::Battery, "battery"),
            (SensorField::LinkQuality, "link_quality"),
            (SensorField::Presence, "presence"),
        ];

        for (field, expected_str) in fields {
            assert_eq!(field.to_db_string(), expected_str);
            assert_eq!(SensorField::from_db_string(expected_str), Some(field));
        }

        assert_eq!(SensorField::from_db_string("invalid"), None);
    }

    #[test]
    fn test_logical_operator_conversions() {
        let operators = vec![(LogicalOperator::And, "and"), (LogicalOperator::Or, "or")];

        for (op, expected_str) in operators {
            assert_eq!(op.to_db_string(), expected_str);
            assert_eq!(LogicalOperator::from_db_string(expected_str), Some(op));
        }

        assert_eq!(LogicalOperator::from_db_string("invalid"), None);
    }

    #[test]
    fn test_condition_variable_target() {
        // Constant condition: no variable target.
        let constant = AutomationCondition::new(
            "sensor1".into(),
            SensorField::Temperature,
            ComparisonOperator::GreaterThan,
            20.0,
        );
        assert!(constant.variable_target().is_none());

        // Variable condition built from a request.
        let cond: AutomationCondition = CreateConditionRequest {
            device_id: "living_room".into(),
            field: SensorField::Temperature,
            operator: ComparisonOperator::GreaterThan,
            value: 0.0,
            target_device_id: Some("bedroom".into()),
            target_field: Some(SensorField::Temperature),
        }
        .into();

        match cond.variable_target() {
            Some((device_id, field)) => {
                assert_eq!(device_id, "bedroom");
                assert_eq!(field, &SensorField::Temperature);
            }
            None => panic!("expected a variable target"),
        }

        // A target device id without a target field is not a variable target.
        let partial = AutomationCondition {
            target_device_id: Some("bedroom".into()),
            target_field: None,
            ..constant.clone()
        };
        assert!(partial.variable_target().is_none());
    }

    #[test]
    fn test_switch_action_conversions() {
        let actions = vec![
            (SwitchAction::On, "on"),
            (SwitchAction::Off, "off"),
            (SwitchAction::Toggle, "toggle"),
        ];

        for (action, expected_str) in actions {
            assert_eq!(action.to_db_string(), expected_str);
            assert_eq!(SwitchAction::from_db_string(expected_str), Some(action));
        }

        assert_eq!(SwitchAction::from_db_string("invalid"), None);
    }
}
