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

impl ComparisonOperator {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            ComparisonOperator::Equal => "eq",
            ComparisonOperator::NotEqual => "neq",
            ComparisonOperator::GreaterThan => "gt",
            ComparisonOperator::GreaterThanOrEqual => "gte",
            ComparisonOperator::LessThan => "lt",
            ComparisonOperator::LessThanOrEqual => "lte",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "eq" => Some(ComparisonOperator::Equal),
            "neq" => Some(ComparisonOperator::NotEqual),
            "gt" => Some(ComparisonOperator::GreaterThan),
            "gte" => Some(ComparisonOperator::GreaterThanOrEqual),
            "lt" => Some(ComparisonOperator::LessThan),
            "lte" => Some(ComparisonOperator::LessThanOrEqual),
            _ => None,
        }
    }

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
}

impl SensorField {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            SensorField::Temperature => "temperature",
            SensorField::Humidity => "humidity",
            SensorField::Battery => "battery",
            SensorField::LinkQuality => "link_quality",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "temperature" => Some(SensorField::Temperature),
            "humidity" => Some(SensorField::Humidity),
            "battery" => Some(SensorField::Battery),
            "link_quality" => Some(SensorField::LinkQuality),
            _ => None,
        }
    }
}

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

    /// Value to compare against
    pub value: f64,
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

impl LogicalOperator {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            LogicalOperator::And => "and",
            LogicalOperator::Or => "or",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "and" => Some(LogicalOperator::And),
            "or" => Some(LogicalOperator::Or),
            _ => None,
        }
    }
}

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

impl SwitchAction {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            SwitchAction::On => "on",
            SwitchAction::Off => "off",
            SwitchAction::Toggle => "toggle",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "on" => Some(SwitchAction::On),
            "off" => Some(SwitchAction::Off),
            "toggle" => Some(SwitchAction::Toggle),
            _ => None,
        }
    }
}

/// An action to execute when rule conditions are met
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateConditionRequest {
    pub device_id: String,
    pub field: SensorField,
    pub operator: ComparisonOperator,
    pub value: f64,
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
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id,
            rule_name,
            success,
            error_message,
            executed_at: Utc::now(),
        }
    }
}
