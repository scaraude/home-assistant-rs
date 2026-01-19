import type { AutomationConditionField } from "./sensors";

export type ComparisonOperator =
  | "equal"
  | "not_equal"
  | "greater_than"
  | "greater_than_or_equal"
  | "less_than"
  | "less_than_or_equal";

export type LogicalOperator = "and" | "or";

export type AutomationActionType = "on" | "off" | "toggle";

export type { AutomationConditionField };
