import type {
  AutomationActionType,
  AutomationConditionField,
  ComparisonOperator,
  LogicalOperator,
} from "../types/automation";
import { parseUnixSeconds, unixSecondsToDate } from "../utils/time";

export type {
  AutomationActionType,
  AutomationConditionField,
  ComparisonOperator,
  LogicalOperator,
};

export interface AutomationCondition {
  id: string;
  device_id: string;
  field: AutomationConditionField;
  operator: ComparisonOperator;
  value: number;
}

export interface AutomationAction {
  id: string;
  device_id: string;
  action: AutomationActionType;
}

export interface AutomationRule {
  id: string;
  name: string;
  description: string | null;
  enabled: boolean;
  condition_operator: LogicalOperator;
  conditions: AutomationCondition[];
  actions: AutomationAction[];
  time_window: TimeWindow;
  created_at: Date;
  updated_at: Date;
  last_triggered_at: Date | null;
  trigger_count: number;
}

export interface TimeWindow {
  enabled: boolean;
  start_time: string | null;
  end_time: string | null;
  active_days: number[] | null;
}

export interface TimeWindowRequest {
  enabled?: boolean;
  start_time?: string;
  end_time?: string;
  active_days?: number[];
}

export interface CreateAutomationRuleRequest {
  name: string;
  description?: string;
  enabled?: boolean;
  condition_operator: LogicalOperator;
  conditions: Array<{
    device_id: string;
    field: AutomationConditionField;
    operator: ComparisonOperator;
    value: number;
  }>;
  actions: Array<{
    device_id: string;
    action: AutomationActionType;
  }>;
  time_window?: TimeWindowRequest;
}

export interface UpdateAutomationRuleRequest {
  name?: string;
  description?: string;
  enabled?: boolean;
  condition_operator?: LogicalOperator;
  conditions?: Array<{
    device_id: string;
    field: AutomationConditionField;
    operator: ComparisonOperator;
    value: number;
  }>;
  actions?: Array<{
    device_id: string;
    action: AutomationActionType;
  }>;
  time_window?: TimeWindowRequest;
}

export interface AutomationExecutionLog {
  id: string;
  rule_id: string;
  rule_name: string;
  success: boolean;
  error_message: string | null;
  executed_at: Date;
}

type AutomationRuleResponse = Omit<
  AutomationRule,
  "created_at" | "updated_at" | "last_triggered_at"
> & {
  created_at: number;
  updated_at: number;
  last_triggered_at: number | null;
};

type AutomationExecutionLogResponse = Omit<AutomationExecutionLog, "executed_at"> & {
  executed_at: number;
};

function parseAutomationRule(rule: AutomationRuleResponse): AutomationRule {
  return {
    ...rule,
    created_at: unixSecondsToDate(rule.created_at),
    updated_at: unixSecondsToDate(rule.updated_at),
    last_triggered_at: parseUnixSeconds(rule.last_triggered_at),
  };
}

function parseAutomationLog(log: AutomationExecutionLogResponse): AutomationExecutionLog {
  return {
    ...log,
    executed_at: unixSecondsToDate(log.executed_at),
  };
}

/**
 * Fetch all automation rules
 */
export async function fetchAutomationRules(): Promise<AutomationRule[]> {
  const response = await fetch('/api/automation/rules');
  if (!response.ok) {
    throw new Error(`Failed to fetch automation rules: ${response.statusText}`);
  }
  const rules = (await response.json()) as AutomationRuleResponse[];
  return rules.map(parseAutomationRule);
}

/**
 * Fetch a single automation rule by ID
 * @param id - Rule ID
 */
export async function fetchAutomationRule(id: string): Promise<AutomationRule> {
  const response = await fetch(`/api/automation/rules/${id}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch automation rule: ${response.statusText}`);
  }
  const rule = (await response.json()) as AutomationRuleResponse;
  return parseAutomationRule(rule);
}

/**
 * Create a new automation rule
 * @param rule - Rule creation request
 */
export async function createAutomationRule(rule: CreateAutomationRuleRequest): Promise<AutomationRule> {
  const response = await fetch('/api/automation/rules', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(rule),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to create automation rule: ${response.statusText}`);
  }

  const created = (await response.json()) as AutomationRuleResponse;
  return parseAutomationRule(created);
}

/**
 * Update an existing automation rule
 * @param id - Rule ID
 * @param updates - Partial rule updates
 */
export async function updateAutomationRule(
  id: string,
  updates: UpdateAutomationRuleRequest
): Promise<AutomationRule> {
  const response = await fetch(`/api/automation/rules/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to update automation rule: ${response.statusText}`);
  }

  const updated = (await response.json()) as AutomationRuleResponse;
  return parseAutomationRule(updated);
}

/**
 * Delete an automation rule
 * @param id - Rule ID
 */
export async function deleteAutomationRule(id: string): Promise<void> {
  const response = await fetch(`/api/automation/rules/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to delete automation rule: ${response.statusText}`);
  }
}

/**
 * Fetch automation execution logs
 * @param limit - Maximum number of log entries (default: 100)
 */
export async function fetchAutomationLogs(limit: number = 100): Promise<AutomationExecutionLog[]> {
  const params = new URLSearchParams();
  params.append('limit', limit.toString());

  const response = await fetch(`/api/automation/logs?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch automation logs: ${response.statusText}`);
  }
  const logs = (await response.json()) as AutomationExecutionLogResponse[];
  return logs.map(parseAutomationLog);
}
