// k6 HTTP smoke for the control plane (not the 1k WS gate).
// Run: k6 run scripts/loadtest/http-smoke.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  vus: 5,
  duration: '10s',
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<500'],
  },
};

const BASE = __ENV.API_BASE || 'http://localhost:8088';

export default function () {
  const health = http.get(`${BASE}/health`);
  check(health, { 'health 200': (r) => r.status === 200 });
  const meta = http.get(`${BASE}/api/v1/meta`);
  check(meta, { 'meta 200': (r) => r.status === 200 });
  sleep(0.2);
}
