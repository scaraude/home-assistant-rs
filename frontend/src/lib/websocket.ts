import { writable, type Readable } from 'svelte/store';
import type { AutomationAction, SensorReading } from './api';

type DeviceCapability =
  | { type: 'sensor'; sensor_type: 'temp_humidity' | 'presence' }
  | { type: 'commander'; commander_type: 'switch' };

export type SensorReadingEvent = {
  event: 'sensor_reading';
  device_id: string;
  reading: SensorReading;
  timestamp: number;
};

export type SwitchStateEvent = {
  event: 'switch_state';
  device_id: string;
  state: boolean | 'ON' | 'OFF';
  timestamp: number;
};

export type DeviceStateEvent = {
  event: 'device_state';
  device_id: string;
  battery: number | null;
  link_quality: number | null;
  timestamp: number;
};

export type DeviceDiscoveredEvent = {
  event: 'device_discovered';
  device_id: string;
  mqtt_topic: string;
  capability: DeviceCapability;
  timestamp: number;
};

export type AutomationTriggeredEvent = {
  event: 'automation_triggered';
  rule_id: string;
  actions: AutomationAction[];
  timestamp: number;
};

export type AutomationExecutedEvent = {
  event: 'automation_executed';
  rule_id: string;
  success: boolean;
  error: string | null;
  timestamp: number;
};

export type SystemEvent =
  | SensorReadingEvent
  | SwitchStateEvent
  | DeviceStateEvent
  | DeviceDiscoveredEvent
  | AutomationTriggeredEvent
  | AutomationExecutedEvent;

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
      const data = JSON.parse(event.data) as SystemEvent;
      this.eventsStore.set(data);
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
