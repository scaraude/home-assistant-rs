/**
 * Runtime-agnostic mock backend core.
 *
 * This holds ALL the fake-backend logic — request routing, in-memory switch
 * state, and the outgoing WebSocket event stream — with zero dependency on
 * Node's `http` types or the browser's `fetch`/`WebSocket`. Two thin adapters
 * consume it:
 *
 *  - `mock/plugin.ts`  — Vite dev-server middleware + `ws` server (npm run dev:mock)
 *  - `mock/browser.ts` — patches window.fetch / window.WebSocket for the static
 *                        online demo (npm run build:demo)
 *
 * Keeping the logic here means the fixtures in `data.ts` and the behaviour of
 * every route are a single source of truth across both runtimes.
 */
import {
  MOCK_DEVICES,
  MOCK_FLOOR_PLAN_SVG,
  MOCK_SWITCHES,
  deviceInfo,
  findSwitch,
  mockPositions,
  mockTopology,
  readingAt,
  seriesFor,
  type MockReading,
} from './data';

export interface MockResponse {
  status: number;
  body: unknown;
  headers?: Record<string, string>;
}

const now = () => Math.floor(Date.now() / 1000);

// --- session-scoped mutable state -------------------------------------------

// Position overrides so dragging a device on the floor plan sticks for the
// session.
const positionOverrides: Record<string, { x: number; y: number }> = {};

// Live on/off state per switch, seeded from the fixtures.
const switchState: Record<string, boolean> = Object.fromEntries(
  MOCK_SWITCHES.map((s) => [s.id, s.initial_state]),
);

// --- outgoing event bus (WebSocket messages) --------------------------------

type Emitter = (message: unknown) => void;
const emitters = new Set<Emitter>();

/** Register a sink for pushed WS messages; returns an unsubscribe function. */
export function onEvent(fn: Emitter): () => void {
  emitters.add(fn);
  return () => emitters.delete(fn);
}

function emit(message: unknown) {
  for (const fn of emitters) fn(message);
}

// --- serializers -------------------------------------------------------------

function deviceStatePayload(deviceId: string, seed: number) {
  return {
    device_id: deviceId,
    battery_level: 60 + (seed % 40),
    link_quality: 120 + (seed % 100),
    last_seen: now() - (seed % 300),
    turbo_mode: null,
  };
}

/** SwitchDevice payload (matches frontend/src/lib/api/switches.ts). */
function switchPayload(id: string) {
  const s = findSwitch(id);
  if (!s) return null;
  return {
    id: s.id,
    name: s.name,
    color: s.color,
    state: switchState[s.id] ?? false,
    link_quality: s.link_quality,
    battery_level: s.battery_level,
    last_seen: now() - (s._seed % 120),
  };
}

// --- readings ----------------------------------------------------------------

function handleReadings(query: URLSearchParams): MockResponse {
  const deviceId = query.get('device_id') ?? undefined;
  const targets = deviceId
    ? MOCK_DEVICES.filter((d) => d.device_id === deviceId)
    : MOCK_DEVICES;

  // latest=1 → one most-recent reading per device.
  if (query.get('latest') === '1' || query.get('latest')?.toLowerCase() === 'true') {
    const t = now();
    const readings = targets.map((d) => readingAt(d, t));
    return { status: 200, body: readings, headers: { 'X-Latest-Timestamp': String(t) } };
  }

  const bucket = Math.max(1, parseInt(query.get('bucket') ?? '1', 10) || 1);
  let start: number;
  let end: number;
  const startParam = query.get('start');
  const endParam = query.get('end');
  if (startParam && endParam) {
    start = parseInt(startParam, 10);
    end = parseInt(endParam, 10);
  } else {
    const hours = parseInt(query.get('hours') ?? '24', 10) || 24;
    end = now();
    start = end - hours * 3600;
  }

  const readings: MockReading[] = [];
  for (const d of targets) {
    readings.push(...seriesFor(d, start, end, bucket));
  }
  const latest = readings.reduce((max, r) => Math.max(max, r.timestamp), end);
  return { status: 200, body: readings, headers: { 'X-Latest-Timestamp': String(latest) } };
}

// --- energy summary ----------------------------------------------------------

const isEnergyMeter = (d: (typeof MOCK_DEVICES)[number]) =>
  d.capabilities.some((c) => c.sensor_type === 'energy_meter');

/**
 * Mirror the Rust `/api/energy/summary`: bucket the cumulative counter and
 * report per-bucket deltas (clamped ≥ 0 so the mock's periodic reset can't go
 * negative), the totals, and peak power.
 */
function handleEnergySummary(query: URLSearchParams): MockResponse {
  const deviceId = query.get('device_id') ?? undefined;
  const bucket = Math.max(1, parseInt(query.get('bucket') ?? '86400', 10) || 86400);
  const end = parseInt(query.get('end') ?? String(now()), 10);
  const start = parseInt(query.get('start') ?? String(end - 86400), 10);

  const meters = (
    deviceId ? MOCK_DEVICES.filter((d) => d.device_id === deviceId) : MOCK_DEVICES
  ).filter(isEnergyMeter);

  const r3 = (x: number) => Math.round(x * 1000) / 1000;
  const agg = new Map<number, { consumed: number; produced: number }>();
  let peakPower = 0;
  let peakTs: number | null = null;

  for (const d of meters) {
    const anchor = readingAt(d, start - 1);
    let prevE = anchor.energy as number;
    let prevP = anchor.produced_energy as number;
    for (let b = start; b < end - 1; b += bucket) {
      const sampleTs = Math.min(b + bucket - 1, end - 1);
      const r = readingAt(d, sampleTs);
      const e = r.energy as number;
      const p = r.produced_energy as number;
      const slot = agg.get(b) ?? { consumed: 0, produced: 0 };
      slot.consumed += Math.max(0, e - prevE);
      slot.produced += Math.max(0, p - prevP);
      agg.set(b, slot);
      prevE = e;
      prevP = p;
      const pw = r.power as number;
      if (pw > peakPower) {
        peakPower = pw;
        peakTs = sampleTs;
      }
    }
  }

  const buckets = [...agg.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([timestamp, v]) => ({ timestamp, consumed: r3(v.consumed), produced: r3(v.produced) }));

  return {
    status: 200,
    body: {
      start,
      end,
      bucket_seconds: bucket,
      total_consumed: r3(buckets.reduce((a, b) => a + b.consumed, 0)),
      total_produced: r3(buckets.reduce((a, b) => a + b.produced, 0)),
      peak_power: Math.round(peakPower * 10) / 10,
      peak_power_timestamp: peakTs,
      buckets,
    },
  };
}

// --- command execution -------------------------------------------------------

/** Toggle a switch, then push a `switch_state` event so the UI reflects it. */
function executeCommand(body: unknown): MockResponse {
  const cmd = (body ?? {}) as { device_id?: string; state?: boolean | 'ON' | 'OFF' };
  const id = cmd.device_id;
  if (!id || !findSwitch(id)) {
    return { status: 404, body: { error: `Unknown switch: ${id ?? '(none)'}` } };
  }
  const desired =
    typeof cmd.state === 'string' ? cmd.state.toUpperCase() === 'ON' : Boolean(cmd.state);
  switchState[id] = desired;

  // Reflect the change on the event stream, like the real backend does after
  // Zigbee confirms the state.
  emit({ event: 'switch_state', device_id: id, state: desired, timestamp: now() });

  return { status: 200, body: { ok: true } };
}

// --- request router ----------------------------------------------------------

/**
 * Handle one `/api/*` request. Pure w.r.t. I/O: takes the parsed method, path,
 * query and (already-parsed) JSON body, returns status + body. Side effects are
 * limited to session state and the event bus.
 */
export function handleRequest(
  method: string,
  path: string,
  query: URLSearchParams,
  body: unknown,
): MockResponse {
  const m = method.toUpperCase();

  // --- reads used by the sensor explorer ---------------------------------
  if (m === 'GET' && path === '/api/sensors') {
    return { status: 200, body: MOCK_DEVICES.map(deviceInfo) };
  }
  if (m === 'GET' && path === '/api/energy/summary') {
    return handleEnergySummary(query);
  }
  if (m === 'GET' && path === '/api/readings') {
    return handleReadings(query);
  }
  if (m === 'GET' && path === '/api/devices/state') {
    return { status: 200, body: MOCK_DEVICES.map((d) => deviceStatePayload(d.device_id, d._seed)) };
  }

  // --- floor plan tab ----------------------------------------------------
  if (m === 'GET' && path === '/api/floor-plan') {
    return { status: 200, body: { svg_content: MOCK_FLOOR_PLAN_SVG, uploaded_at: now() } };
  }
  if (m === 'GET' && path === '/api/network/topology') {
    return { status: 200, body: mockTopology() };
  }
  if (m === 'GET' && path === '/api/devices/positions') {
    return { status: 200, body: mockPositions(positionOverrides) };
  }
  if (m === 'PUT' && /^\/api\/devices\/.+\/position$/.test(path)) {
    const deviceId = path.slice('/api/devices/'.length, -'/position'.length);
    const { x, y } = (body ?? {}) as { x?: number; y?: number };
    if (typeof x === 'number' && typeof y === 'number') {
      positionOverrides[deviceId] = { x, y };
    }
    return { status: 200, body: { ok: true } };
  }

  // --- commander tab: real switch list + state mutation ------------------
  if (m === 'GET' && path === '/api/devices/switches') {
    return { status: 200, body: MOCK_SWITCHES.map((s) => switchPayload(s.id)) };
  }
  if (m === 'POST' && path === '/api/commands/execute') {
    return executeCommand(body);
  }

  // --- other tabs: minimal but non-breaking answers ----------------------
  if (m === 'GET' && path === '/api/automation/rules') return { status: 200, body: [] };
  if (m === 'GET' && path === '/api/automation/logs') return { status: 200, body: [] };
  if (m === 'GET' && path === '/api/system/storage') return { status: 200, body: {} };
  if (m === 'GET' && path.startsWith('/api/logs/')) return { status: 200, body: [] };

  // --- writes: accept and echo success -----------------------------------
  if (m === 'POST' && path === '/api/zigbee/permit_join') return { status: 200, body: { ok: true } };
  if (m === 'POST' && path === '/api/network/refresh') return { status: 200, body: { ok: true } };
  if (m === 'DELETE' && path === '/api/floor-plan') return { status: 200, body: { ok: true } };
  if (m === 'PATCH' && path.startsWith('/api/devices/')) return { status: 200, body: { ok: true } };
  if (m === 'PUT' && path.startsWith('/api/devices/')) return { status: 200, body: { ok: true } };

  // Anything else under /api: safe empty payload so the UI never hard-fails.
  return { status: 200, body: [] };
}

// --- pushed sensor readings --------------------------------------------------

let tickIndex = 0;

/**
 * Build the next live `sensor_reading` message for a rotating device. Callers
 * own the interval (dev server ties it to the http server lifecycle, browser
 * ties it to the socket), so the core stays timer-free.
 */
export function nextSensorReading() {
  const dev = MOCK_DEVICES[tickIndex % MOCK_DEVICES.length];
  tickIndex += 1;
  const t = now();
  return {
    event: 'sensor_reading',
    device_id: dev.device_id,
    reading: readingAt(dev, t),
    timestamp: t,
  };
}
