/**
 * Vite dev-server plugin that fakes the Rust backend so the frontend can run
 * fully standalone. Enabled only when VITE_MOCK is set (see `npm run dev:mock`).
 *
 *  - Intercepts every `/api/*` request and answers via the shared mock core.
 *  - Runs a WebSocket server on `/ws` that pushes live sensor readings and
 *    switch-state changes, so the "latest value" chips, sync indicator and
 *    Commander tab behave like production.
 *
 * All the actual behaviour lives in `mock/core.ts` (shared with the in-browser
 * demo shim). This file is just the Node/Vite adapter and is only imported by
 * vite.config.ts, wired in when the env flag is on — it never touches the
 * production bundle.
 */
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { Plugin, ViteDevServer } from 'vite';
import { WebSocketServer, type WebSocket } from 'ws';
import { handleRequest, nextSensorReading, onEvent } from './core';

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

function send(res: ServerResponse, method: string, path: string, query: URLSearchParams, body: unknown) {
  const { status, body: out, headers } = handleRequest(method, path, query, body);
  res.statusCode = status;
  res.setHeader('Content-Type', 'application/json');
  for (const [k, v] of Object.entries(headers ?? {})) res.setHeader(k, v);
  res.end(JSON.stringify(out));
}

function handleApi(req: IncomingMessage, res: ServerResponse): boolean {
  const url = new URL(req.url ?? '/', 'http://localhost');
  if (!url.pathname.startsWith('/api/')) return false;
  const method = (req.method ?? 'GET').toUpperCase();

  // Reads carry no body; writes need it parsed first.
  if (method === 'GET' || method === 'DELETE') {
    send(res, method, url.pathname, url.searchParams, null);
  } else {
    readBody(req, (body) => send(res, method, url.pathname, url.searchParams, body));
  }
  return true;
}

function setupWebSocket(server: ViteDevServer) {
  const httpServer = server.httpServer;
  if (!httpServer) return;

  const wss = new WebSocketServer({ noServer: true });
  const clients = new Set<WebSocket>();

  const broadcast = (message: unknown) => {
    const payload = JSON.stringify(message);
    for (const ws of clients) {
      if (ws.readyState === ws.OPEN) ws.send(payload);
    }
  };

  httpServer.on('upgrade', (req, socket, head) => {
    const url = new URL(req.url ?? '/', 'http://localhost');
    // Only claim /ws — let Vite's own HMR socket handle everything else.
    if (url.pathname !== '/ws') return;
    wss.handleUpgrade(req, socket as never, head, (ws) => {
      clients.add(ws);
      ws.on('close', () => clients.delete(ws));
    });
  });

  // Relay events pushed by the core (e.g. switch_state after a command).
  const unsubscribe = onEvent(broadcast);

  // Push a live sensor reading for a rotating device every few seconds.
  const timer = setInterval(() => {
    if (clients.size === 0) return;
    broadcast(nextSensorReading());
  }, 3000);

  httpServer.on('close', () => {
    clearInterval(timer);
    unsubscribe();
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
