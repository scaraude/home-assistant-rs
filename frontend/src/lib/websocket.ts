import { writable, type Readable } from 'svelte/store';
import type { AutomationAction, SensorReading, SensorReadingResponse, LogEntry } from './api';
import { parseSensorReading } from './api';
import type { DeviceCapability } from './api/devices';
import { unixSecondsToDate } from './utils/time';

export type LogFile = 'system_monitor' | 'process_monitor' | 'top_cpu_consumers' | 'top_ram_consumers';

export type SensorReadingEvent = {
  event: 'sensor_reading';
  device_id: string;
  reading: SensorReading;
  timestamp: Date;
};

export type SwitchStateEvent = {
  event: 'switch_state';
  device_id: string;
  state: boolean | 'ON' | 'OFF';
  timestamp: Date;
};

export type DeviceStateEvent = {
  event: 'device_state';
  device_id: string;
  battery: number | null;
  link_quality: number | null;
  turbo_mode: boolean | null;
  timestamp: Date;
};

export type DeviceDiscoveredEvent = {
  event: 'device_discovered';
  device_id: string;
  mqtt_topic: string;
  capabilities: DeviceCapability[];
  timestamp: Date;
};

export type AutomationTriggeredEvent = {
  event: 'automation_triggered';
  rule_id: string;
  actions: AutomationAction[];
  timestamp: Date;
};

export type AutomationExecutedEvent = {
  event: 'automation_executed';
  rule_id: string;
  success: boolean;
  error: string | null;
  timestamp: Date;
};

export type LogEntriesEvent = {
  event: 'log_entries';
  log_file: LogFile;
  entries: LogEntry[];
  timestamp: Date;
};

export type SystemEvent =
  | SensorReadingEvent
  | SwitchStateEvent
  | DeviceStateEvent
  | DeviceDiscoveredEvent
  | AutomationTriggeredEvent
  | AutomationExecutedEvent
  | LogEntriesEvent;

type SensorReadingEventPayload = Omit<SensorReadingEvent, "reading" | "timestamp"> & {
  reading: SensorReadingResponse;
  timestamp: number;
};

type SwitchStateEventPayload = Omit<SwitchStateEvent, "timestamp"> & { timestamp: number };
type DeviceStateEventPayload = Omit<DeviceStateEvent, "timestamp"> & { timestamp: number };
type DeviceDiscoveredEventPayload = Omit<DeviceDiscoveredEvent, "timestamp"> & { timestamp: number };
type AutomationTriggeredEventPayload = Omit<AutomationTriggeredEvent, "timestamp"> & { timestamp: number };
type AutomationExecutedEventPayload = Omit<AutomationExecutedEvent, "timestamp"> & { timestamp: number };
type LogEntriesEventPayload = Omit<LogEntriesEvent, "timestamp"> & { timestamp: number };

type SystemEventPayload =
  | SensorReadingEventPayload
  | SwitchStateEventPayload
  | DeviceStateEventPayload
  | DeviceDiscoveredEventPayload
  | AutomationTriggeredEventPayload
  | AutomationExecutedEventPayload
  | LogEntriesEventPayload;

function parseSystemEvent(event: SystemEventPayload): SystemEvent {
  switch (event.event) {
    case "sensor_reading":
      return {
        ...event,
        reading: parseSensorReading(event.reading),
        timestamp: unixSecondsToDate(event.timestamp),
      };
    case "switch_state":
    case "device_state":
    case "device_discovered":
    case "automation_triggered":
    case "automation_executed":
    case "log_entries":
      return {
        ...event,
        timestamp: unixSecondsToDate(event.timestamp),
      };
    default: {
      const _exhaustive: never = event;
      return _exhaustive;
    }
  }
}

class EventStreamClient {
  private socket: WebSocket | null = null;
  private reconnectAttempts = 0;
  private manualClose = false;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly eventsStore = writable<SystemEvent | null>(null);

  public readonly events: Readable<SystemEvent | null> = {
    subscribe: this.eventsStore.subscribe,
  };

  connect() {
    if (typeof window === 'undefined') {
      return;
    }

    if (
      this.socket &&
      (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING)
    ) {
      this.manualClose = false;
      return;
    }

    this.manualClose = false;
    this.startConnection(0);
  }

  disconnect() {
    this.manualClose = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.socket && this.socket.readyState !== WebSocket.CLOSED) {
      this.socket.close();
    }
    this.socket = null;
    this.reconnectAttempts = 0;
  }

  private startConnection(delayMs: number) {
    if (this.manualClose) {
      return;
    }

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }

    this.reconnectTimer = setTimeout(() => {
      this.openSocket();
    }, delayMs);
  }

  private openSocket() {
    if (this.manualClose) {
      return;
    }

    const url = this.buildUrl();

    try {
      const socket = new WebSocket(url);
      this.socket = socket;

      socket.onopen = () => {
        this.reconnectAttempts = 0;
      };

      socket.onmessage = (event: MessageEvent<string>) => {
        this.handleMessage(event);
      };

      socket.onerror = (error) => {
        console.error('WebSocket error', error);
      };

      socket.onclose = () => {
        this.socket = null;
        if (!this.manualClose) {
          this.scheduleReconnect();
        }
      };
    } catch (error) {
      console.error('Failed to open WebSocket connection', error);
      this.scheduleReconnect();
    }
  }

  private handleMessage(event: MessageEvent<string>) {
    try {
      const data = JSON.parse(event.data) as SystemEventPayload;
      this.eventsStore.set(parseSystemEvent(data));
    } catch (error) {
      console.error('Invalid WebSocket payload', error);
    }
  }

  private scheduleReconnect() {
    if (this.manualClose) {
      return;
    }
    const delay = Math.min(1000 * 2 ** this.reconnectAttempts, 30000);
    this.reconnectAttempts += 1;
    this.startConnection(delay);
  }

  private buildUrl(): string {
    const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
    return `${protocol}://${window.location.host}/ws`;
  }
}

export const eventStream = new EventStreamClient();
