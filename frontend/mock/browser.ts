/**
 * In-browser mock backend for the static online demo.
 *
 * The Vite dev plugin (`mock/plugin.ts`) runs in Node and is NOT part of the
 * production bundle, so a statically-hosted build (e.g. GitHub Pages) has
 * nothing answering `/api/*` or `/ws`. This module fills that gap entirely
 * client-side by patching `window.fetch` and `window.WebSocket`, delegating to
 * the same shared core so the behaviour matches `npm run dev:mock`.
 *
 * It is imported dynamically from `main.ts` only when built with
 * `VITE_MOCK_BUILD=1` (see `npm run build:demo`), so a normal production build
 * that talks to the real Rust backend never pulls it in.
 */
import { handleRequest, nextSensorReading, onEvent } from './core';

const API_PREFIX = '/api/';
const WS_PATH = '/ws';

function isApiPath(pathname: string): boolean {
  return pathname.startsWith(API_PREFIX);
}

/** Patch fetch so `/api/*` requests are served from the core in-memory. */
function installFetch() {
  const realFetch = window.fetch.bind(window);

  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const request = input instanceof Request ? input : null;
    const rawUrl = request ? request.url : String(input);
    const url = new URL(rawUrl, window.location.origin);

    if (!isApiPath(url.pathname)) {
      return realFetch(input as RequestInfo, init);
    }

    const method = (init?.method ?? request?.method ?? 'GET').toUpperCase();

    let body: unknown = null;
    const rawBody = init?.body ?? (request ? await request.clone().text() : null);
    if (typeof rawBody === 'string' && rawBody.length > 0) {
      try {
        body = JSON.parse(rawBody);
      } catch {
        body = null;
      }
    }

    const { status, body: out, headers } = handleRequest(method, url.pathname, url.searchParams, body);

    return new Response(JSON.stringify(out), {
      status,
      headers: { 'Content-Type': 'application/json', ...(headers ?? {}) },
    });
  };
}

type Listener = (ev: Event) => void;

/**
 * Minimal WebSocket stand-in for the `/ws` event stream. Implements just enough
 * of the WebSocket surface that EventStreamClient relies on (readyState, the
 * static constants, onopen/onmessage/onclose, close()).
 */
class MockWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  url: string;

  onopen: Listener | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: Listener | null = null;
  onclose: Listener | null = null;

  private timer: ReturnType<typeof setInterval> | null = null;
  private unsubscribe: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    // Open on the next tick so the caller can attach handlers first.
    setTimeout(() => this.open(), 0);
  }

  private open() {
    if (this.readyState !== MockWebSocket.CONNECTING) return;
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.(new Event('open'));

    // Relay events pushed by the core (switch_state after a command).
    this.unsubscribe = onEvent((message) => this.deliver(message));

    // Push a live sensor reading for a rotating device every few seconds.
    this.timer = setInterval(() => this.deliver(nextSensorReading()), 3000);
  }

  private deliver(message: unknown) {
    if (this.readyState !== MockWebSocket.OPEN) return;
    this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(message) }));
  }

  send() {
    // The frontend never sends over this socket; nothing to do.
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.timer) clearInterval(this.timer);
    if (this.unsubscribe) this.unsubscribe();
    this.timer = null;
    this.unsubscribe = null;
    this.onclose?.(new CloseEvent('close'));
  }

  addEventListener() {}
  removeEventListener() {}
}

/** Patch WebSocket so `/ws` connections are served locally; others pass through. */
function installWebSocket() {
  const RealWebSocket = window.WebSocket;

  const Patched = function (this: unknown, url: string | URL, protocols?: string | string[]) {
    const target = new URL(String(url), window.location.origin);
    if (target.pathname === WS_PATH) {
      return new MockWebSocket(String(url)) as unknown as WebSocket;
    }
    return new RealWebSocket(url, protocols);
  } as unknown as typeof WebSocket;

  Patched.prototype = RealWebSocket.prototype;
  Object.defineProperties(Patched, {
    CONNECTING: { value: 0 },
    OPEN: { value: 1 },
    CLOSING: { value: 2 },
    CLOSED: { value: 3 },
  });

  window.WebSocket = Patched;
}

/** Install the in-browser mock backend. Idempotent-ish; call once at startup. */
export function installMockBackend() {
  installFetch();
  installWebSocket();
  // eslint-disable-next-line no-console
  console.log('🧪 Demo mode — /api and /ws are served in-browser (no backend).');
}
