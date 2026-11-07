// Exercise the candidate's real HTTPS adapters without starting a desktop host.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { setTimeout as pause } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const repo = fileURLToPath(new URL('../../', import.meta.url));
const exe = process.env.ATMOS_NATIVE_CANDIDATE || path.join(repo, 'desktop-host/target/debug/atmos-wallpaper.exe');
const temp = fs.realpathSync(os.tmpdir());
const scratch = fs.mkdtempSync(path.join(temp, 'atmos-native-live-'));
const store = path.join(scratch, 'store');
const rss = 'https://feeds.bbci.co.uk/news/rss.xml';
const started = Date.now();
const child = spawn(exe, [
    '--native-runtime-worker', store, path.join(repo, 'implementation-p4_2_5/dist'),
    path.join(repo, 'prototype-v8.1-frozen'), process.env.ATMOS_PYTHON || 'C:/Python314/python.exe',
    path.join(repo, 'implementation-p1/src/decode-image.py'), process.pid + '-' + started,
], {
    windowsHide: true,
    env: { ...process.env, PATH: '', LOCALAPPDATA: path.join(scratch, 'local'), NODE_OPTIONS: '--invalid-fixture-option' },
    stdio: ['pipe', 'pipe', 'pipe'],
});
const closed = once(child, 'close');
const lines = createInterface({ input: child.stdout });
let stderr = '';
child.stderr.setEncoding('utf8').on('data', chunk => { stderr = (stderr + chunk).slice(-4096); });
const deadline = setTimeout(() => child.kill(), 55000);
const nav = { 'Sec-Fetch-Mode': 'navigate', 'Sec-Fetch-Dest': 'document', 'Sec-Fetch-Site': 'none' };

function request(url, headers, body) {
    return new Promise((resolve, reject) => {
        const req = http.request(url, { method: body ? 'POST' : 'GET', headers, agent: false }, res => {
            let text = '';
            res.setEncoding('utf8').on('data', chunk => { text += chunk; });
            res.on('error', reject);
            res.on('end', () => resolve({ status: res.statusCode, text }));
        });
        req.on('error', reject);
        req.setTimeout(10000, () => req.destroy(Error('Loopback request timed out')));
        req.end(body);
    });
}

try {
    const ready = once(lines, 'line');
    child.stdin.write('start\n');
    const [line] = await Promise.race([ready, closed.then(([code]) => { throw Error('Worker exited ' + code + ': ' + stderr); })]);
    const { origin } = JSON.parse(line);
    assert.match(origin, /^http:\/\/127\.0\.0\.1:\d+$/);
    const page = await request(origin + '/manager', nav);
    assert.equal(page.status, 200);
    const token = page.text.match(/atmosBridgeSession="([a-f0-9]{64})"/)?.[1];
    assert.ok(token);
    async function api(route, body) {
        const headers = { 'Sec-Fetch-Site': 'same-origin', 'X-Atmos-Session': token };
        const payload = body ? JSON.stringify(body) : undefined;
        if (payload) Object.assign(headers, { Origin: origin, 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(payload) });
        const response = await request(origin + route, headers, payload);
        const value = JSON.parse(response.text);
        assert.equal(response.status, 200, route + ' ' + JSON.stringify(value.error));
        return value;
    }
    const daily = '/api/daily-feeds';
    const providers = '/api/providers/settings';
    const [feeds, weather] = await Promise.all([api(daily), api(providers)]);
    const cards = feeds.settings.cards;
    await api(daily, { schemaVersion: 1, expectedRevision: feeds.settings.revision,
        cards, rss: { ...feeds.settings.rss, enabled: true, url: rss } });
    await api(providers, { schemaVersion: 1, expectedRevision: weather.settings.revision,
        requestId: 'live-weather', weather: { enabled: true, city: '東京' } });
    async function refreshedFeeds() {
        let results;
        for (let attempt = 0; attempt < 30; attempt++) {
            results = (await api(daily)).feeds;
            if (Object.values(results).every(feed => !feed.loading && (feed.fetchedAtMs || feed.error))) break;
            await pause(500);
        }
        return results;
    }
    const [observed, results] = await Promise.all([api('/api/providers/weather'), refreshedFeeds()]);
    const summary = {
        elapsedMs: Date.now() - started,
        weather: {
            passed: observed.available === true && observed.stale === false &&
                observed.data?.fetchedAtMs >= started && Number.isFinite(observed.data?.temperatureC),
            error: observed.error?.code ?? null,
        },
        feeds: Object.fromEntries(Object.entries(results).map(([id, feed]) => [id, {
            passed: feed.enabled && !feed.loading && !feed.stale && feed.fetchedAtMs >= started && feed.items.length > 0,
            count: feed.items.length, error: feed.error?.code ?? null,
        }])),
    };
    console.log(JSON.stringify(summary));
    assert.ok(summary.weather.passed && Object.keys(summary.feeds).length === 3 &&
        Object.values(summary.feeds).every(feed => feed.passed), 'One or more live sources failed');
} finally {
    child.stdin.end();
    const cleanup = setTimeout(() => child.kill(), 3000);
    await closed.catch(() => {});
    clearTimeout(cleanup);
    clearTimeout(deadline);
    lines.close();
    const resolved = fs.realpathSync(scratch);
    assert.equal(path.dirname(resolved), temp);
    fs.rmSync(resolved, { recursive: true, force: true });
}
