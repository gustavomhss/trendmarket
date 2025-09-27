import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend } from 'k6/metrics';

export const options = {
  vus: Number(__ENV.K6_VUS || 5),
  duration: __ENV.K6_DURATION || '30s',
  thresholds: {
    http_req_duration: ['p(95)<800', 'p(99)<1200'],
    checks: ['rate>0.99'],
  },
  summaryTrendStats: ['min', 'avg', 'med', 'p(90)', 'p(95)', 'p(99)'],
};

const DEFAULT_BASE_URL = 'http://localhost:8080';
const DEFAULT_QUOTE_ENDPOINT = '/pricing/quote';

const baseUrl = __ENV.K6_BASE_URL || DEFAULT_BASE_URL;
const quoteEndpoint = __ENV.K6_QUOTE_ENDPOINT || DEFAULT_QUOTE_ENDPOINT;

function assertDefaultQuotePath(base, endpoint) {
  if (base === DEFAULT_BASE_URL && endpoint === DEFAULT_QUOTE_ENDPOINT) {
    const expectedUrl = `${DEFAULT_BASE_URL}${DEFAULT_QUOTE_ENDPOINT}`;
    const actualUrl = `${base.replace(/\/$/, '')}${endpoint}`;

    if (actualUrl !== expectedUrl) {
      throw new Error(
        `Expected default quote URL to be ${expectedUrl}, but received ${actualUrl}`,
      );
    }
  }
}

assertDefaultQuotePath(baseUrl, quoteEndpoint);
const latency = new Trend('decision_latency', true);

function buildQuotePayload() {
  return {
    app_id: 'smoke-app-001',
    applicant: {
      document: '00000000191',
      income: 7200,
      score: 712,
    },
    product: {
      code: 'PX-001',
      amount: 5000,
      tenor_months: 12,
    },
    channel: 'ops-smoke-k6',
  };
}

function requestQuote() {
  const url = `${baseUrl.replace(/\/$/, '')}${quoteEndpoint}`;
  const payload = JSON.stringify(buildQuotePayload());
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'X-Feature-Flag': __ENV.K6_FEATURE_FLAG || 'ops-smoke',
    },
    timeout: __ENV.K6_TIMEOUT || '5s',
  };

  const response = http.post(url, payload, params);
  latency.add(response.timings.duration);

  const ok = check(response, {
    'status is 2xx': (r) => r.status >= 200 && r.status < 300,
    'has price payload': (r) => !!r.json('price.apr'),
    'latency under p95 budget': (r) => r.timings.duration <= Number(__ENV.K6_LATENCY_BUDGET_MS || 800),
  });

  if (!ok) {
    console.error(`Quote smoke failure: status=${response.status} body=${response.body}`);
  }
}

export default function smoke() {
  requestQuote();
  sleep(Number(__ENV.K6_SLEEP_SECONDS || 1));
}
