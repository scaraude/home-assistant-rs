/**
 * Fake data generators for local development (see mock/plugin.ts).
 *
 * Everything here is DETERMINISTIC for a given (deviceId, timestamp): the same
 * timestamp always yields the same value. This matters because the frontend
 * caches readings by time-range and refetches overlapping ranges — a
 * non-deterministic generator would make points "jump" between requests.
 *
 * Response shapes mirror the Rust backend exactly (see src/models/*.rs and
 * src/http/routes/sensors.rs):
 *  - DeviceInfo: { device_id, name, capabilities[], available_fields[], color }
 *  - SensorReading: internally tagged by "type", timestamp is unix SECONDS.
 */

export type SensorType = 'temp_humidity' | 'presence' | 'energy_meter';

export interface MockDevice {
  device_id: string;
  name: string;
  capabilities: Array<{ type: 'sensor'; sensor_type: SensorType }>;
  available_fields: string[];
  color: string | null;
  // generation parameters (not serialized)
  _seed: number;
  _base?: number; // temp baseline / power baseline
  _amp?: number; // diurnal amplitude
}

const AVAILABLE_FIELDS: Record<SensorType, string[]> = {
  temp_humidity: ['temperature', 'humidity'],
  presence: ['presence', 'illumination'],
  energy_meter: [
    'power',
    'voltage',
    'current',
    'energy',
    'produced_energy',
    'ac_frequency',
    'power_factor',
  ],
};

function device(
  device_id: string,
  name: string,
  sensor_type: SensorType,
  color: string,
  seed: number,
  base?: number,
  amp?: number,
): MockDevice {
  return {
    device_id,
    name,
    capabilities: [{ type: 'sensor', sensor_type }],
    available_fields: AVAILABLE_FIELDS[sensor_type],
    color,
    _seed: seed,
    _base: base,
    _amp: amp,
  };
}

/** Catalogue of fake devices, one per (sensor type, room). */
export const MOCK_DEVICES: MockDevice[] = [
  // temp / humidity
  device('0xmock000000000t1', 'Salon', 'temp_humidity', '#d62728', 11, 21, 2.5),
  device('0xmock000000000t2', 'Chambre', 'temp_humidity', '#1f77b4', 22, 19, 1.8),
  device('0xmock000000000t3', 'Extérieur', 'temp_humidity', '#2ca02c', 33, 12, 9),
  device('0xmock000000000t4', 'Cuisine', 'temp_humidity', '#9467bd', 44, 22, 3),
  // presence
  device('0xmock000000000p1', 'Présence salon', 'presence', '#ffa600', 55),
  device('0xmock000000000p2', 'Présence entrée', 'presence', '#ff7f0e', 66),
  device('0xmock000000000p3', 'Présence bureau', 'presence', '#e377c2', 77),
  // energy
  device('0xmock000000000e1', 'Compteur général', 'energy_meter', '#17becf', 88, 220),
  device('0xmock000000000e2', 'Prise bureau', 'energy_meter', '#8c564b', 99, 15),
];

const DAY = 86400;

/** Deterministic pseudo-random in [0, 1) from an integer key. */
function rand(key: number): number {
  const x = Math.sin(key * 12.9898) * 43758.5453;
  return x - Math.floor(x);
}

function hourOfDay(ts: number): number {
  return ((ts % DAY) + DAY) % DAY / 3600; // 0..24
}

/** Diurnal curve peaking mid-afternoon (~15h), trough pre-dawn (~4h). */
function diurnal(ts: number): number {
  const h = hourOfDay(ts);
  return Math.sin(((h - 9) / 24) * 2 * Math.PI);
}

function temperature(dev: MockDevice, ts: number): number {
  const base = dev._base ?? 21;
  const amp = dev._amp ?? 2;
  const noise = (rand(Math.floor(ts / 600) + dev._seed) - 0.5) * 0.7;
  return round(base + amp * diurnal(ts) + noise, 1);
}

function humidity(dev: MockDevice, ts: number): number {
  // Inversely correlated with temperature, room-dependent offset.
  const noise = (rand(Math.floor(ts / 600) + dev._seed * 3) - 0.5) * 4;
  const v = 55 - 12 * diurnal(ts) + (dev._seed % 7) - 3 + noise;
  return round(clamp(v, 28, 92), 1);
}

/** 30-minute occupancy blocks so bands read as contiguous, not confetti. */
function occupied(dev: MockDevice, ts: number): boolean {
  const block = Math.floor(ts / 1800) + dev._seed;
  const h = hourOfDay(ts);
  // Room-specific likelihood by time of day.
  let prob: number;
  if (dev.device_id.endsWith('p2')) {
    // entrance: short spikes, mostly morning/evening
    prob = h > 7 && h < 9 ? 0.5 : h > 18 && h < 21 ? 0.5 : 0.05;
  } else if (dev.device_id.endsWith('p3')) {
    // office: daytime working hours
    prob = h > 9 && h < 18 ? 0.6 : 0.03;
  } else {
    // living room: evenings + weekend-ish daytime
    prob = h > 17 && h < 24 ? 0.75 : h > 8 && h < 12 ? 0.4 : 0.05;
  }
  return rand(block) < prob;
}

function illumination(ts: number): string {
  const h = hourOfDay(ts);
  return h > 8 && h < 19 ? 'bright' : 'dark';
}

function power(dev: MockDevice, ts: number): number {
  const base = dev._base ?? 30;
  const h = hourOfDay(ts);
  const dayMask = h > 7 && h < 23 ? 1 : 0.3;
  const slot = Math.floor(ts / 900) + dev._seed; // 15-min slots
  const r = rand(slot);
  let spike = 0;
  if (r > 0.85) spike = r * (dev._base && dev._base > 100 ? 1800 : 400);
  else if (r > 0.6) spike = r * (dev._base && dev._base > 100 ? 300 : 60);
  const noise = rand(slot * 7) * 8;
  return round(Math.max(0, base + spike * dayMask + noise), 1);
}

function round(v: number, digits = 2): number {
  const f = 10 ** digits;
  return Math.round(v * f) / f;
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, v));
}

/** A serialized reading matching SensorReadingResponse on the frontend. */
export type MockReading = Record<string, unknown> & { type: SensorType; timestamp: number };

export function readingAt(dev: MockDevice, ts: number): MockReading {
  const sensorType = dev.capabilities[0].sensor_type;
  switch (sensorType) {
    case 'temp_humidity':
      return {
        type: 'temp_humidity',
        device_id: dev.device_id,
        temperature: temperature(dev, ts),
        humidity: humidity(dev, ts),
        timestamp: ts,
      };
    case 'presence':
      return {
        type: 'presence',
        device_id: dev.device_id,
        occupied: occupied(dev, ts),
        illumination: illumination(ts),
        timestamp: ts,
      };
    case 'energy_meter': {
      const p = power(dev, ts);
      const voltage = round(230 + (rand(ts + dev._seed) - 0.5) * 6, 1);
      return {
        type: 'energy_meter',
        device_id: dev.device_id,
        power: p,
        // Cumulative-ish energy that grows through the month then resets, so it
        // stays in a plausible range instead of exploding with the unix epoch.
        energy: round(((ts % (DAY * 30)) / DAY) * (dev._base ?? 30) * 0.02, 3),
        produced_energy: 0,
        voltage,
        current: round(p / voltage, 3),
        ac_frequency: 50,
        power_factor: round(clamp(0.85 + rand(ts) * 0.14, 0, 1), 3),
        timestamp: ts,
      };
    }
  }
}

/**
 * Generate a bucketed series for a device between [start, end] (unix seconds).
 * Step is at least `bucket` (>= 60s), capped so a single response never
 * exceeds ~2000 points — same spirit as the server-side aggregation.
 */
export function seriesFor(dev: MockDevice, start: number, end: number, bucket: number): MockReading[] {
  const MAX_POINTS = 2000;
  let step = Math.max(bucket || 60, 60);
  const span = Math.max(0, end - start);
  if (span / step > MAX_POINTS) {
    step = Math.ceil(span / MAX_POINTS);
  }
  const out: MockReading[] = [];
  const first = Math.ceil(start / step) * step;
  for (let ts = first; ts <= end; ts += step) {
    out.push(readingAt(dev, ts));
  }
  return out;
}

export function findDevice(deviceId: string): MockDevice | undefined {
  return MOCK_DEVICES.find((d) => d.device_id === deviceId);
}

// ---------------------------------------------------------------------------
// Floor plan: a fake apartment SVG + a star network topology + device
// positions, so the Floor Plan tab is populated in mock mode too.
// ---------------------------------------------------------------------------

/** A neutral apartment layout (viewBox 0 0 900 700). */
export const MOCK_FLOOR_PLAN_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 700" width="900" height="700">
  <defs>
    <pattern id="terrace" width="12" height="12" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
      <line x1="0" y1="0" x2="0" y2="12" stroke="#bbf7d0" stroke-width="4" />
    </pattern>
  </defs>
  <!-- rooms -->
  <g fill="#f1f5f9" stroke="#94a3b8" stroke-width="3">
    <rect x="20" y="20" width="450" height="280" />
    <rect x="20" y="300" width="450" height="300" />
    <rect x="470" y="20" width="410" height="300" />
    <rect x="470" y="320" width="230" height="280" />
    <rect x="700" y="320" width="180" height="280" />
  </g>
  <!-- terrace -->
  <rect x="40" y="614" width="360" height="70" fill="#f0fdf4" stroke="#86efac" stroke-width="2" stroke-dasharray="8 6" />
  <rect x="40" y="614" width="360" height="70" fill="url(#terrace)" opacity="0.5" />
  <!-- outer wall -->
  <rect x="20" y="20" width="860" height="580" fill="none" stroke="#475569" stroke-width="6" />
  <!-- door openings (cover wall strokes) -->
  <g fill="#f1f5f9">
    <rect x="200" y="297" width="70" height="6" />
    <rect x="467" y="150" width="6" height="70" />
    <rect x="467" y="420" width="6" height="70" />
    <rect x="697" y="430" width="6" height="70" />
    <rect x="300" y="597" width="70" height="6" />
  </g>
  <!-- windows on exterior walls -->
  <g stroke="#60a5fa" stroke-width="5">
    <line x1="120" y1="20" x2="230" y2="20" />
    <line x1="600" y1="20" x2="740" y2="20" />
    <line x1="880" y1="120" x2="880" y2="220" />
    <line x1="120" y1="600" x2="240" y2="600" />
  </g>
  <!-- labels -->
  <g fill="#475569" font-family="system-ui, sans-serif" font-size="22" font-weight="600" text-anchor="middle">
    <text x="245" y="165">Cuisine</text>
    <text x="245" y="455">Salon</text>
    <text x="675" y="175">Chambre</text>
    <text x="585" y="465">Bureau</text>
    <text x="790" y="465">Entrée</text>
  </g>
  <text x="220" y="656" fill="#4d7c0f" font-family="system-ui, sans-serif" font-size="18" font-weight="600" text-anchor="middle">Terrasse</text>
</svg>`;

const COORDINATOR_ID = '0xmock0000000cord';

/** Device positions on the plan (top-left anchor, SVG coordinates). */
const POSITIONS: Record<string, { x: number; y: number }> = {
  [COORDINATOR_ID]: { x: 390, y: 40 },
  '0xmock000000000t1': { x: 180, y: 410 }, // Salon
  '0xmock000000000t2': { x: 610, y: 130 }, // Chambre
  '0xmock000000000t3': { x: 180, y: 620 }, // Extérieur (terrasse)
  '0xmock000000000t4': { x: 180, y: 120 }, // Cuisine
  '0xmock000000000p1': { x: 260, y: 470 }, // Présence salon
  '0xmock000000000p2': { x: 740, y: 430 }, // Présence entrée
  '0xmock000000000p3': { x: 520, y: 430 }, // Présence bureau
  '0xmock000000000e1': { x: 730, y: 520 }, // Compteur général (entrée)
  '0xmock000000000e2': { x: 560, y: 370 }, // Prise bureau
};

/** Network topology: a coordinator with every sensor attached in a star. */
export function mockTopology() {
  const coordinator = {
    id: COORDINATOR_ID,
    mqtt_topic: 'coordinator',
    ieee_addr: COORDINATOR_ID,
    name: 'Coordinateur',
    capabilities: [{ type: 'coordinator' as const }],
    available_fields: [] as string[],
    power_source: 'plugged' as const,
    added_at: 1_700_000_000,
    is_bridge: true,
    parent_device_id: null,
    color: '#334155',
  };

  const devices = MOCK_DEVICES.map((d) => {
    const sensorType = d.capabilities[0].sensor_type;
    return {
      id: d.device_id,
      mqtt_topic: d.name,
      ieee_addr: d.device_id,
      name: d.name,
      capabilities: d.capabilities,
      available_fields: d.available_fields,
      power_source: (sensorType === 'energy_meter' ? 'plugged' : 'battery') as
        | 'plugged'
        | 'battery',
      added_at: 1_700_000_000 + d._seed * 1000,
      is_bridge: false,
      parent_device_id: COORDINATOR_ID,
      color: d.color,
    };
  });

  const edges = MOCK_DEVICES.map((d) => ({
    source_id: COORDINATOR_ID,
    target_id: d.device_id,
    link_quality: 110 + (d._seed % 100),
  }));

  return { devices: [coordinator, ...devices], edges };
}

/** Default device positions payload. */
export function mockPositions(overrides: Record<string, { x: number; y: number }> = {}) {
  const updated = Math.floor(Date.now() / 1000);
  return Object.entries({ ...POSITIONS, ...overrides }).map(([device_id, p]) => ({
    device_id,
    x: p.x,
    y: p.y,
    updated_at: updated,
  }));
}

/** DeviceInfo payload (strip generation params). */
export function deviceInfo(dev: MockDevice) {
  return {
    device_id: dev.device_id,
    name: dev.name,
    capabilities: dev.capabilities,
    available_fields: dev.available_fields,
    color: dev.color,
  };
}
