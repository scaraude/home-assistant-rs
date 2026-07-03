/**
 * Vite dev-server plugin that fakes the Rust backend so the frontend can run
 * fully standalone. Enabled only when VITE_MOCK is set (see `npm run dev:mock`).
 *
 *  - Intercepts every `/api/*` request and answers with generated JSON.
 *  - Runs a WebSocket server on `/ws` that pushes live sensor readings, so the
 *    "latest value" chips and sync indicator behave like production.
 *
 * It touches nothing in the production bundle: this file is only imported by
 * vite.config.ts and only wired in when the env flag is on.
 */
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { Plugin, ViteDevServer } from 'vite';
import { WebSocketServer, type WebSocket } from 'ws';
import {
  MOCK_DEVICES,
  MOCK_FLOOR_PLAN_SVG,
  deviceInfo,
  mockPositions,
  mockTopology,
  readingAt,
  seriesFor,
  type MockReading,
} from './data';

// Session-scoped position overrides so dragging a device on the floor plan
// sticks until the dev server restarts.
const positionOverrides: Record<string, { x: number; y: number }> = {};

const now = () => Math.floor(Date.now() / 1000);

function json(res: ServerResponse, body: unknown, extraHeaders: Record<string, string> = {}) {
  const payload = JSON.stringify(body);
  res.statusCode = 200;
  res.setHeader('Content-Type', 'application/json');
  for (const [k, v] of Object.entries(extraHeaders)) res.setHeader(k, v);
  res.end(payload);
}

function readBody(req: IncomingMessage, cb: (body: unknown) => void) {
  let raw = '';
  req.on('data', (chunk) => (raw += chunk));
  req.on('end', () => {
    try {
      cb(raw ? JSON.parse(raw) : null);
    } catch {
      cb(null);
    }
  });
}

function deviceState(deviceId: string, seed: number) {
  return {
    device_id: deviceId,
    battery_level: 60 + (seed % 40),
    link_quality: 120 + (seed % 100),
    last_seen: now() - (seed % 300),
    turbo_mode: null,
  };
}

function handleReadings(url: URL, res: ServerResponse) {
  const q = url.searchParams;
  const deviceId = q.get('device_id') ?? undefined;
  const targets = deviceId ? MOCK_DEVICES.filter((d) => d.device_id === deviceId) : MOCK_DEVICES;

  // latest=1 → one most-recent reading per device.
  if (q.get('latest') === '1' || q.get('latest')?.toLowerCase() === 'true') {
    const t = now();
    const readings = targets.map((d) => readingAt(d, t));
    return json(res, readings, { 'X-Latest-Timestamp': String(t) });
  }

  const bucket = Math.max(1, parseInt(q.get('bucket') ?? '1', 10) || 1);
  let start: number;
  let end: number;
  const startParam = q.get('start');
  const endParam = q.get('end');
  if (startParam && endParam) {
    start = parseInt(startParam, 10);
    end = parseInt(endParam, 10);
  } else {
    const hours = parseInt(q.get('hours') ?? '24', 10) || 24;
    end = now();
    start = end - hours * 3600;
  }

  const readings: MockReading[] = [];
  for (const d of targets) {
    readings.push(...seriesFor(d, start, end, bucket));
  }
  const latest = readings.reduce((max, r) => Math.max(max, r.timestamp), end);
  json(res, readings, { 'X-Latest-Timestamp': String(latest) });
}

function handleApi(req: IncomingMessage, res: ServerResponse): boolean {
  const url = new URL(req.url ?? '/', 'http://localhost');
  const path = url.pathname;
  const method = (req.method ?? 'GET').toUpperCase();

  if (!path.startsWith('/api/')) return false;

  // --- reads used by the sensor explorer -------------------------------
  if (method === 'GET' && path === '/api/sensors') {
    return json(res, MOCK_DEVICES.map(deviceInfo)), true;
  }
  if (method === 'GET' && path === '/api/readings') {
    return handleReadings(url, res), true;
  }
  if (method === 'GET' && path === '/api/devices/state') {
    return json(res, MOCK_DEVICES.map((d) => deviceState(d.device_id, d._seed))), true;
  }

  // --- floor plan tab --------------------------------------------------
  if (method === 'GET' && path === '/api/floor-plan') {
    return json(res, { svg_content: MOCK_FLOOR_PLAN_SVG, uploaded_at: now() }), true;
  }
  if (method === 'GET' && path === '/api/network/topology') {
    return json(res, mockTopology()), true;
  }
  if (method === 'GET' && path === '/api/devices/positions') {
    return json(res, mockPositions(positionOverrides)), true;
  }
  // Persist a dragged device position for this session.
  if (method === 'PUT' && /^\/api\/devices\/.+\/position$/.test(path)) {
    const deviceId = path.slice('/api/devices/'.length, -'/position'.length);
    readBody(req, (body) => {
      const { x, y } = (body ?? {}) as { x?: number; y?: number };
      if (typeof x === 'number' && typeof y === 'number') {
        positionOverrides[deviceId] = { x, y };
      }
      json(res, { ok: true });
    });
    return true;
  }

  // --- startup / other tabs: minimal but non-breaking answers ----------
  if (method === 'GET' && path === '/api/devices/switches') return json(res, []), true;
  if (method === 'GET' && path === '/api/automation/rules') return json(res, []), true;
  if (method === 'GET' && path === '/api/automation/logs') return json(res, []), true;
  if (method === 'GET' && path === '/api/system/storage') return json(res, {}), true;
  if (method === 'GET' && path.startsWith('/api/logs/')) return json(res, []), true;

  // --- writes: accept and echo success --------------------------------
  if (method === 'POST' && path === '/api/commands/execute') return json(res, { ok: true }), true;
  if (method === 'POST' && path === '/api/zigbee/permit_join') return json(res, { ok: true }), true;
  if (method === 'POST' && path === '/api/network/refresh') return json(res, { ok: true }), true;
  if (method === 'DELETE' && path === '/api/floor-plan') return json(res, { ok: true }), true;
  if (method === 'PATCH' && path.startsWith('/api/devices/')) return json(res, { ok: true }), true;
  if (method === 'PUT' && path.startsWith('/api/devices/')) return json(res, { ok: true }), true;

  // Anything else under /api: safe empty payload so the UI never hard-fails.
  json(res, []);
  return true;
}

function setupWebSocket(server: ViteDevServer) {
  const httpServer = server.httpServer;
  if (!httpServer) return;

  const wss = new WebSocketServer({ noServer: true });
  const clients = new Set<WebSocket>();

  httpServer.on('upgrade', (req, socket, head) => {
    const url = new URL(req.url ?? '/', 'http://localhost');
    // Only claim /ws — let Vite's own HMR socket handle everything else.
    if (url.pathname !== '/ws') return;
    wss.handleUpgrade(req, socket as never, head, (ws) => {
      clients.add(ws);
      ws.on('close', () => clients.delete(ws));
    });
  });

  // Push a live reading for a rotating device every few seconds.
  let i = 0;
  const timer = setInterval(() => {
    if (clients.size === 0) return;
    const dev = MOCK_DEVICES[i % MOCK_DEVICES.length];
    i += 1;
    const t = now();
    const message = JSON.stringify({
      event: 'sensor_reading',
      device_id: dev.device_id,
      reading: readingAt(dev, t),
      timestamp: t,
    });
    for (const ws of clients) {
      if (ws.readyState === ws.OPEN) ws.send(message);
    }
  }, 3000);

  httpServer.on('close', () => {
    clearInterval(timer);
    wss.close();
  });
}

export function mockBackend(): Plugin {
  return {
    name: 'mock-backend',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        try {
          if (!handleApi(req, res)) next();
        } catch (err) {
          // eslint-disable-next-line no-console
          console.error('[mock] error handling', req.url, err);
          res.statusCode = 500;
          res.end('mock error');
        }
      });
      setupWebSocket(server);
      // eslint-disable-next-line no-console
      console.log('\n  🧪  Mock backend active — /api and /ws are served locally.\n');
    },
  };
}
