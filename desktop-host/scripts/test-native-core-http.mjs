import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import os from 'node:os';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { createHostBridge } from '../../implementation-p4_2_5/src/host-router.mjs';
import { createDisplayService } from '../../implementation-p4_2_5/src/display-service.mjs';
import { createLayoutService } from '../../implementation-p4_2_5/src/layout-service.mjs';
import { createStartupService } from '../../implementation-p4_2_5/src/startup-service.mjs';
import { createAudioService } from '../../implementation-p4_2_5/src/audio-service.mjs';
import { createPerformanceService } from '../../implementation-p4_2_5/src/performance-service.mjs';
import { createLibraryService } from '../../implementation-p4_2_5/src/library-service.mjs';
import { createProvidersService } from '../../implementation-p4_2_5/src/providers-service.mjs';
import { createDailyFeeds } from '../../implementation-p4_2_5/src/daily-feeds.mjs';
import { createCalendarService } from '../../implementation-p4_2_5/src/calendar-service.mjs';
import { Store } from '../../implementation-p4_2_5/src/portable-store.mjs';
import { LibraryManager } from '../../implementation-p4_2_5/src/library-core.mjs';
import { hash } from '../../implementation-p4_2_5/src/contract.mjs';
import { exportRenderer } from '../../implementation-p4_2_5/src/export-renderer.mjs';

const repo = fileURLToPath(new URL('../../', import.meta.url));
const app = path.join(repo, 'implementation-p4_2_5');
const prototype = path.join(repo, 'prototype-v8.1-frozen');
const exe = process.env.ATMOS_CORE_HTTP_CANDIDATE ||
    path.join(repo, 'desktop-host/target/debug/atmos-wallpaper.exe');
const python = process.env.ATMOS_PYTHON || 'C:/Python314/python.exe';
const left = 'display-' + 'a'.repeat(64);
const right = 'display-' + 'b'.repeat(64);
const offline = 'display-' + 'c'.repeat(64);
const displays = [
    { id: left, deviceName: '\\\\.\\DISPLAY1', primary: true, bounds: { x: 0, y: 0, width: 1920, height: 1080 }, scalePercent: 100 },
    { id: right, deviceName: '\\\\.\\DISPLAY2', primary: false, bounds: { x: -1080, y: -350, width: 1080, height: 1920 }, scalePercent: 150 },
].map((d, i) => ({ ...d, name: d.deviceName, number: i + 1, selectable: true, identityConfidence: 'fixture' }));

function put(root, name, value) {
    fs.writeFileSync(path.join(root, name), JSON.stringify(value));
}
function read(root, name) {
    return JSON.parse(fs.readFileSync(path.join(root, name), 'utf8'));
}
const command = (requestId, expectedRevision, background) => ({
    schemaVersion: 1, type: 'settings.update', requestId, expectedRevision, changes: { visual: { background } },
});

test('native host controls enforce roles, shape, CAS and display freshness over HTTP', async t => {
    const f = await fixture(t);
    const host = (name, options) => f.call('/api/host/' + name, options);
    const post = async (name, body, status = 200) => result(await host(name, { body }), status);
    const expectCode = async (name, body, status, code) => assert.equal((await post(name, body, status)).error.code, code);
    const revision = { schemaVersion: 1, expectedRevision: 0 };
    for (const name of ['displays', 'layout', 'startup', 'audio', 'audio/routing', 'audio/test', 'performance', 'library']) {
        const denied = await result(await host(name, { wallpaper: true }), 403);
        assert.equal(denied.error.code, 'ROLE_DENIED');
    }
    assert.equal((await result(await host('startup', { body: { enabled: true }, headers: { Origin: null } }), 403)).error.code, 'ORIGIN_REQUIRED');
    assert.equal((await result(await host('layout', { body: 'x'.repeat(4096) }), 413)).error.code, 'BODY_TOO_LARGE');
    assert.equal((await result(await host('startup', { body: { enabled: true }, headers: { 'Content-Type': 'text/plain' } }), 415)).error.code, 'CONTENT_TYPE_DENIED');
    const readDisplay = async () => result(await host('displays'));
    assert.equal((await readDisplay()).displays[1].id, right);
    put(f.root, 'host-status.json', { capturedAtMs: Date.now() });
    assert.equal((await readDisplay()).runtimeFresh, true);
    for (const delta of [-6000, 6000]) {
        put(f.root, 'host-status.json', { capturedAtMs: Date.now() + delta });
        assert.equal((await readDisplay()).runtime, null);
    }
    await expectCode('displays', { ...revision, placement: { enabled: true, target: offline } }, 400, 'TARGET_OFFLINE_OR_AMBIGUOUS');
    assert.equal((await post('displays', { ...revision, placement: { enabled: true, target: right } })).settings.revision, 1);
    await expectCode('displays', { ...revision, placement: { enabled: true, target: left } }, 409, 'HOST_REVISION_CONFLICT');
    const multi = { ...revision, requestId: 'layout-1', layout: { mode: 'independent', targets: [left, right], askOnApply: false } };
    assert.equal((await post('layout', multi)).result.committed, true);
    assert.equal((await post('layout', multi)).result.replayed, true);
    await expectCode('layout', { ...multi, layout: { ...multi.layout, askOnApply: true } }, 409, 'LAYOUT_REQUEST_CONFLICT');
    await expectCode('displays', { ...revision, expectedRevision: 1, placement: { enabled: true, target: left } }, 409, 'HOST_LAYOUT_ACTIVE');
    assert.deepEqual(await post('startup', { enabled: true }), { available: true, enabled: true, scope: 'current-user', trigger: 'logon' });
    assert.equal((await result(await host('startup'))).enabled, true);
    await expectCode('startup', { enabled: false, command: 'unexpected' }, 400, 'STARTUP_INVALID');

    const audio = { ...revision, requestId: 'audio-1', audio: { enabled: true, muted: true, focusOnly: true, volume: 60, source: 'rain', output: 'system-default' } };
    assert.equal((await post('audio', audio)).settings.revision, 1);
    assert.equal((await post('audio', audio)).result.replayed, true);
    await expectCode('audio/test', revision, 409, 'AUDIO_REVISION_CONFLICT');
    await expectCode('audio/test', { ...revision, expectedRevision: 1 }, 400, 'AUDIO_TEST_BLOCKED');
    assert.equal(fs.existsSync(path.join(f.root, 'audio-test.json')), false);
    put(f.root, 'audio-status.json', { pid: process.pid, capturedAtMs: Date.now() - 4000 });
    assert.equal((await result(await host('audio'))).runtime, null);
    const route = { ...revision, requestId: 'route-1', routing: { mode: 'primary' } };
    await expectCode('audio/routing', route, 400, 'AUDIO_ROUTING_INVALID');
    assert.equal((await post('audio/routing', { ...route, routing: { mode: 'display', target: right } })).settings.routing.target, right);
    const performance = { ...revision, requestId: 'performance-1', performance: { fps: 15, informationMode: true, pauseBattery: false, pauseFullscreen: true } };
    assert.equal((await post('performance', performance)).settings.performance.informationMode, true);
    assert.equal((await post('performance', { ...performance, requestId: 'performance-legacy', expectedRevision: 1, performance: { fps: 5, pauseBattery: false, pauseFullscreen: false } })).settings.performance.informationMode, true);
    await expectCode('performance', { ...performance, requestId: 'performance-bad', performance: { ...performance.performance, fps: 60 } }, 400, 'PERFORMANCE_INVALID');
    assert.equal(read(f.root, 'live.json').revision, 0);
});

test('native library status, open job and scoped read-only system telemetry', async t => {
    const f = await fixture(t);
    const library = '/api/host/library';
    const post = async (body, status) => result(await f.call(library, { body }), status);
    const base = { schemaVersion: 1, expectedRevision: 0, requestId: 'library-1' };
    const assets = path.join(f.root, 'assets');
    const status = await result(await f.call(library));
    assert.equal(status.path, assets);
    assert.equal(status.online, true);
    assert.equal(status.assetCount, 0);
    assert.equal(status.job, null);
    assert.equal((await post({ ...base, action: 'open', path: 'C:\\' }, 400)).error.code, 'LIBRARY_COMMAND_INVALID');
    assert.equal((await post({ ...base, action: 'open', expectedRevision: 2 }, 409)).error.code, 'LIBRARY_REVISION_CONFLICT');
    assert.equal((await post({ ...base, action: 'archive', auditHash: 'bad' }, 400)).error.code, 'LIBRARY_COMMAND_INVALID');
    const queued = await post({ ...base, action: 'open' }, 202);
    assert.equal(queued.state, 'queued');
    let completed;
    for (let attempt = 0; attempt < 20; attempt++) {
        completed = (await result(await f.call(library))).job;
        if (completed.state === 'completed') break;
        await new Promise(resolve => setTimeout(resolve, 20));
    }
    assert.equal(completed.id, queued.id);
    assert.deepEqual(completed.result, { opened: true, committed: false });
    const relocated = path.join(path.dirname(f.root), 'absent-library');
    put(f.root, 'library-config.json', { schemaVersion: 1, revision: 1, root: relocated, id: 'a'.repeat(36) });
    const offlineLibrary = await result(await f.call(library));
    assert.equal(offlineLibrary.online, false);
    assert.equal(offlineLibrary.error.code, 'LIBRARY_OFFLINE');
    assert.equal(offlineLibrary.path, relocated);
    assert.equal(fs.existsSync(relocated), false);

    const endpoint = '/api/providers/system';
    assert.equal((await result(await f.call(endpoint, { body: {} }), 405)).error.code, 'METHOD_DENIED');
    const first = await result(await f.call(endpoint, { wallpaper: true }));
    assert.equal(first.source, 'windows-native-readonly');
    assert.equal(first.available, true);
    assert.equal(first.stale, false);
    assert.deepEqual(first.disks.map(disk => disk.drive), ['C:', 'D:']);
    assert.equal(first.power, null);
    assert.equal(first.gpu, null);
    const second = await result(await f.call(endpoint));
    assert.equal(second.disksSampledAtMs, first.disksSampledAtMs);
    f.layout('independent', [left, right]);
    assert.equal((await result(await f.call(endpoint), 409)).error.code, 'TARGET_REQUIRED');
    const scoped = await result(await f.call(endpoint, { wallpaper: true, headers: scope(right) }));
    assert.equal(scoped.disksSampledAtMs, first.disksSampledAtMs);
    assert.equal(fs.existsSync(path.join(f.root, 'targets', right)), false);
});

function request(url, { method = 'GET', headers = {}, body } = {}) {
    return new Promise((resolve, reject) => {
        const request = http.request(url, { method, headers, agent: false }, response => {
            const chunks = [];
            response.on('data', chunk => chunks.push(chunk));
            response.on('error', reject);
            response.on('end', () => resolve(new Response(
                response.statusCode === 304 ? null : Buffer.concat(chunks),
                { status: response.statusCode, headers: response.headers },
            )));
        });
        request.on('error', reject);
        // Import may invoke the bounded 15-second image decoder before replying.
        request.setTimeout(25000, () => request.destroy(new Error('Fixture request timed out: ' + new URL(url).pathname)));
        request.end(body);
    });
}

test('native host worker starts without Node and is reaped on exit or process termination', async t => {
    const tempRoot = fs.realpathSync(os.tmpdir());
    const tmp = fs.mkdtempSync(path.join(tempRoot, 'atmos-native-host-test-'));
    const store = path.join(tmp, 'store');
    put(tmp, 'displays.json', displays);
    const workerGone = async pid => {
        for (let attempt = 0; attempt < 100; attempt++) {
            try { process.kill(pid, 0); } catch (error) {
                assert.equal(error.code, 'ESRCH');
                return;
            }
            await new Promise(resolve => setTimeout(resolve, 25));
        }
        assert.fail('Managed native worker still running');
    };
    t.after(() => {
        const resolved = fs.realpathSync(tmp);
        assert.equal(path.dirname(resolved), tempRoot);
        fs.rmSync(resolved, { recursive: true, force: true });
    });
    let previousSession;
    for (const mode of ['normal', 'kill-host', 'kill-worker']) {
        const child = spawn(exe, ['--native-runtime-host-fixture', store, path.join(app, 'dist'),
            prototype, path.join(tmp, 'displays.json'), python, path.join(repo, 'implementation-p1/src/decode-image.py')], {
            windowsHide: true,
            env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
            stdio: ['pipe', 'pipe', 'pipe'],
        });
        const closed = once(child, 'close');
        let stderr = '';
        child.stderr.on('data', chunk => { stderr += chunk; });
        let ready;
        try {
            let output = '';
            const line = new Promise((resolve, reject) => {
                child.stdout.on('data', chunk => {
                    output += chunk;
                    if (output.includes('\n')) {
                        try { resolve(JSON.parse(output.split('\n')[0])); } catch (error) { reject(error); }
                    }
                });
            });
            ready = await Promise.race([line, closed.then(([code]) => {
                throw Error(`Managed host exited ${code}: ${stderr}`);
            })]);
            assert.equal(ready.hostPid, child.pid);
            assert.ok(ready.workerPid > 0 && ready.workerPid !== child.pid && ready.workerPid !== process.pid);
            assert.match(ready.origin, /^http:\/\/127\.0\.0\.1:\d+$/);
            assert.match(ready.sessionId, new RegExp('^' + child.pid + '-[0-9]+$'));
            assert.notEqual(ready.sessionId, previousSession);
            previousSession = ready.sessionId;
            const page = await request(ready.origin + '/manager', { headers: {
                'Sec-Fetch-Mode': 'navigate', 'Sec-Fetch-Dest': 'document', 'Sec-Fetch-Site': 'none',
            } });
            assert.equal(page.status, 200);
            const session = (await page.text()).match(/atmosBridgeSession="([a-f0-9]{64})"/)[1];
            const headers = { 'Sec-Fetch-Site': 'same-origin', 'X-Atmos-Session': session };
            const health = await result(await request(ready.origin + '/api/health', { headers }));
            assert.equal(health.schemaVersion, 1);
            assert.equal(health.rendererRecentlyAcknowledged, false);
            assert.equal(health.nativeAllAttached, false);
            if (mode === 'normal') child.stdin.end();
            if (mode === 'kill-host') assert.equal(child.kill(), true);
            if (mode === 'kill-worker') process.kill(ready.workerPid);
            const [code, signal] = await closed;
            if (mode === 'normal') assert.equal(code, 0, stderr);
            if (mode === 'kill-host') assert.equal(signal, 'SIGTERM');
            if (mode === 'kill-worker') {
                assert.equal(code, 2);
                assert.match(stderr, /NATIVE_WORKER_EXITED/);
            }
            await workerGone(ready.workerPid);
            await assert.rejects(request(ready.origin + '/api/health', { headers }), { code: 'ECONNREFUSED' });
        } finally {
            child.kill();
            await closed;
            if (ready?.hostPid === child.pid && ready.workerPid !== child.pid) await workerGone(ready.workerPid);
        }
    }
});

async function fixture(t, referenceServices) {
    const tempRoot = fs.realpathSync(os.tmpdir());
    const tmp = fs.mkdtempSync(path.join(tempRoot, 'atmos-core-http-test-'));
    const root = path.join(tmp, 'native');
    const legacy = path.join(tmp, 'legacy');
    put(tmp, 'displays.json', displays);
    let child, closed;
    async function stop(crash = false) {
        if (!child) return;
        if (crash) child.kill();
        child.stdin.end();
        const kill = setTimeout(() => child.kill(), 5000);
        try { await closed; } finally { clearTimeout(kill); child = null; }
    }
    t.after(async () => {
        await stop();
        const resolved = fs.realpathSync(tmp);
        assert.equal(path.dirname(resolved), tempRoot);
        await fs.promises.rm(resolved, { recursive: true, force: true, maxRetries: 8, retryDelay: 200 });
    });
    async function start() {
        child = spawn(exe, ['--core-http-fixture', root, path.join(app, 'dist'), prototype, path.join(tmp, 'displays.json'),
            python, path.join(repo,'implementation-p1/src/decode-image.py')], {
            windowsHide: true,
            env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
            stdio: ['pipe', 'pipe', 'pipe'],
        });
        closed = once(child, 'close');
        let output = '';
        let errors = '';
        child.stderr.on('data', chunk => { errors += chunk; });
        const ready = await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => reject(new Error('Fixture did not start')), 15000);
            child.once('error', e => { clearTimeout(timeout); reject(e); });
            child.once('exit', code => { clearTimeout(timeout); reject(new Error(`Fixture exited ${code}: ${errors}`)); });
            child.stdout.on('data', chunk => {
                output += chunk;
                if (output.includes('\n')) {
                    clearTimeout(timeout);
                    try { resolve(JSON.parse(output.split('\n')[0])); } catch (e) { reject(e); }
                }
            });
        });
        assert.equal(ready.syntheticOnly, true);
        return ready.origin;
    }
    let origin = await start();
    const navigation = { 'Sec-Fetch-Mode': 'navigate', 'Sec-Fetch-Dest': 'document', 'Sec-Fetch-Site': 'none' };
    const page = route => request(origin + route, { headers: navigation });
    const token = async route => {
        const response = await page(route);
        assert.equal(response.status, 200);
        return (await response.text()).match(/atmosBridgeSession="([a-f0-9]{64})"/)[1];
    };
    let manager = await token('/manager');
    let wallpaper = await token('/wallpaper');
    async function restart() {
        await stop(true);
        origin = await start();
        manager = await token('/manager');
        wallpaper = await token('/wallpaper');
    }
    const referenceOrigin = 'http://127.0.0.1:12345';
    const bridge = createHostBridge({
        store: legacy, origin: referenceOrigin, staticRoot: path.join(app, 'dist'),
        prototypeRoot: prototype, hostSessionId: 'synthetic-reference',
        coreOptions: { pythonExecutable: python },
        displayService: { latest: () => ({ displays, settings: { placement: { enabled: true, target: 'primary' } } }) },
        ...referenceServices?.(legacy),
    });
    async function call(route, options = {}, reference = false) {
        const body = options.body === undefined ? undefined : JSON.stringify(options.body);
        const host = reference ? referenceOrigin : origin;
        const session = options.wallpaper
            ? (reference ? bridge.wallpaperSessionToken : wallpaper)
            : (reference ? bridge.sessionToken : manager);
        const headers = {
            'Sec-Fetch-Site': 'same-origin', 'X-Atmos-Session': session,
            ...(body === undefined ? {} : { Origin: host, 'Content-Type': 'application/json' }),
            ...options.headers,
        };
        for (const key of Object.keys(headers)) if (headers[key] === null) delete headers[key];
        const method = options.method || (body === undefined ? 'GET' : 'POST');
        return reference
            ? bridge.handle(new Request(host + route, { method, headers: { ...headers, Host: '127.0.0.1:12345' }, body }), { rawPath: route })
            : request(host + route, { method, headers, body });
    }
    function layout(mode, targets, revision = 1) {
        for (const store of [root, legacy]) put(store, 'display-layout.json', {
            schemaVersion: 1, revision, lastRequestId: null, layout: { mode, targets, askOnApply: false },
        });
    }
    return { call, page, layout, root, legacy, bridge, restart, get origin() { return origin; } };
}

test('hidden WebView2 runs the real manager and wallpaper renderers', async t => {
    const f = await fixture(t);
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !/^WEBVIEW2_/i.test(key)));
    const child = spawn(exe, ['--native-runtime-webview-fixture', f.root, f.origin], {
        windowsHide: true,
        env: { ...env, LOCALAPPDATA: path.join(path.dirname(f.root), 'local') },
        stdio: ['pipe', 'ignore', 'pipe'],
    });
    const closed = once(child, 'close');
    let errors = '';
    child.stderr.setEncoding('utf8').on('data', chunk => { errors = (errors + chunk).slice(-4096); });
    try {
        let health;
        for (let attempt = 0; attempt < 100; attempt++) {
            assert.equal(child.exitCode, null, errors);
            health = await result(await f.call('/api/health'));
            if (health.rendererRecentlyAcknowledged &&
                health.renderers.some(r => r.role === 'manager' && r.recent)) break;
            await new Promise(resolve => setTimeout(resolve, 200));
        }
        assert.equal(health.rendererRecentlyAcknowledged, true, JSON.stringify(health));
        assert.ok(health.renderers.some(r => r.role === 'manager' && r.recent));
        assert.equal(health.nativeAllAttached, false);
        t.diagnostic('Both WebView2 renderers acknowledged the candidate; desktop attachment was not exercised.');
    } finally {
        child.stdin.end();
        const timeout = setTimeout(() => child.kill(), 5000);
        try {
            const [code] = await closed;
            assert.equal(code, 0, errors);
        } finally {
            clearTimeout(timeout);
        }
    }
});

const scope = (target, scopeId = target) => ({ 'X-Atmos-Target': target, 'X-Atmos-Scope': scopeId });
async function result(response, status = 200) {
    const value = await response.json();
    assert.equal(response.status, status, JSON.stringify(value));
    return value;
}

test('forced termination recovers library copy, pointer and archive transactions', async t => {
    async function cli(f, action, input) {
        const run = spawnSync(exe, ['--library-command', action, '--store', f.root], {
            input: input === undefined ? undefined : JSON.stringify(input), encoding: 'utf8', windowsHide: true,
            timeout: 10000, env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
        });
        assert.ifError(run.error);
        assert.equal(run.status, 0, run.stdout || run.stderr);
        return JSON.parse(run.stdout).result;
    }
    async function crash(f, phase, action, input) {
        const child = spawn(exe, ['--library-crash-fixture', f.root, phase, action], {
            stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
            env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
        });
        const closed = once(child, 'close');
        let stdout = '', stderr = '';
        child.stdout.on('data', chunk => { stdout += chunk; });
        child.stderr.on('data', chunk => { stderr += chunk; });
        child.stdin.end(JSON.stringify(input));
        try {
            const deadline = Date.now() + 10000;
            while (!stdout.includes('\n') && child.exitCode === null && Date.now() < deadline) {
                await new Promise(resolve => setTimeout(resolve, 15));
            }
            assert.deepEqual(JSON.parse(stdout.split('\n')[0] || '{}'), { checkpoint: phase, pid: child.pid },
                stdout || stderr || 'worker did not reach crash checkpoint');
            assert.equal(child.kill(), true);
            const [exit] = await closed;
            assert.notEqual(exit, 0);
            assert.notEqual(exit, 99, 'fixture timer expired before forced termination');
        } finally {
            if (child.exitCode === null && child.signalCode === null) child.kill();
            await closed;
        }
    }
    const f = await fixture(t);
    const data = Buffer.from('retained during crash recovery');
    const id = hash(data);
    const asset = path.join(f.root, 'assets', id + '.png');
    fs.writeFileSync(asset, data);
    put(path.dirname(asset), id + '.json', { assetId: id, bytes: data.length, format: 'PNG', width: 1, height: 1 });
    const move = { schemaVersion: 1, expectedRevision: 0, requestId: 'crash-move' };
    await crash(f, 'move-after-copy', 'reset', move);
    assert.equal((await cli(f, 'get')).revision, 0);
    assert.equal(fs.readdirSync(f.root + '-library/assets').length, 1);
    await crash(f, 'move-before-switch', 'reset', move);
    assert.equal((await cli(f, 'get')).revision, 0);
    const completed = await cli(f, 'reset', move);
    assert.equal(completed.revision, 1);
    assert.deepEqual(fs.readFileSync(asset), data);

    const audit = await cli(f, 'audit');
    const archive = { schemaVersion: 1, expectedRevision: 1, requestId: 'crash-archive', auditHash: audit.auditHash };
    await crash(f, 'archive-after-remove', 'archive', archive);
    assert.equal(read(f.root, 'library-archive-intent.json').state, 'pending');
    assert.equal(fs.readdirSync(path.join(completed.path, 'assets')).length, 1);
    const restored = await cli(f, 'get');
    assert.equal(restored.assetCount, 1);
    assert.equal(read(f.root, 'library-archive-intent.json').state, 'aborted');
    assert.deepEqual(fs.readFileSync(path.join(completed.path, 'assets', id + '.png')), data);
    const archived = await cli(f, 'archive', archive);
    assert.equal(archived.archivedAssets, 1);
    assert.deepEqual(fs.readFileSync(path.join(archived.archivePath, id + '.png')), data);

    const after = await fixture(t);
    const switched = { schemaVersion: 1, expectedRevision: 0, requestId: 'crash-after-switch' };
    await crash(after, 'move-after-switch', 'reset', switched);
    assert.equal((await cli(after, 'get')).revision, 1);
    const queried = await cli(after, 'result', { schemaVersion: 1, requestId: switched.requestId });
    assert.equal(queried.result.recovered, true);
    const replay = await cli(after, 'reset', switched);
    assert.equal(replay.replayed, true);
    assert.equal(replay.revision, 1);
    assert.equal((await cli(after, 'get')).revision, 1);
});

test('native library audit and copy-verify-switch preserve source and legacy receipts', async t => {
    const f = await fixture(t);
    const library = '/api/host/library';
    const post = async (body, status) => result(await f.call(library, { body }), status);
    const read = async () => result(await f.call(library));
    const base = { schemaVersion: 1, expectedRevision: 0, requestId: 'library-test' };
    const assets = path.join(f.root, 'assets');
    const data = Buffer.from('content-addressed test data');
    const id = hash(data);
    fs.writeFileSync(path.join(assets, id + '.png'), data);
    put(assets, id + '.json', { assetId: id, bytes: data.length, format: 'PNG', width: 1, height: 1 });
    async function job(requestId) {
        for (let i = 0; i < 60; i++) {
            const status = await read();
            if (status.job?.requestId === requestId && ['completed', 'failed', 'cancelled'].includes(status.job.state)) return status.job;
            await new Promise(resolve => setTimeout(resolve, 35));
        }
        assert.fail('library job did not settle');
    }
    await post({ ...base, action: 'choose', requestId: 'choose-test' }, 202);
    assert.equal((await job('choose-test')).state, 'cancelled');

    await post({ ...base, action: 'audit', requestId: 'audit-test' }, 202);
    const audited = await job('audit-test');
    assert.equal(audited.state, 'completed', JSON.stringify(audited.error));
    const legacyManager = new LibraryManager(f.root);
    const { files, ...expectedAudit } = legacyManager.cleanupAudit();
    assert.deepEqual(audited.result.audit, expectedAudit);

    const moveId = 'relocate-test';
    await post({ ...base, action: 'reset', requestId: moveId }, 202);
    const moved = await job(moveId);
    assert.equal(moved.state, 'completed', JSON.stringify(moved.error));
    assert.equal(moved.result.revision, 1);
    assert.equal(moved.result.sourceRetained, true);
    assert.deepEqual(fs.readFileSync(path.join(assets, id + '.png')), data);
    assert.deepEqual(fs.readFileSync(path.join(moved.result.path, 'assets', id + '.png')), data);
    assert.deepEqual(legacyManager.recoverReceipt(moveId), moved.result);
    assert.equal(legacyManager.move(moved.result.path, { expectedRevision: 0, requestId: moveId }).replayed, true);
    assert.equal((await read()).isDefault, true);

    function cli(action, value) {
        const child = spawnSync(exe, ['--library-command', action, '--store', f.root], {
            input: value === undefined ? undefined : JSON.stringify(value), encoding: 'utf8', windowsHide: true,
            env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
        });
        assert.ifError(child.error);
        assert.equal(child.status, 0, child.stdout || child.stderr);
        return JSON.parse(child.stdout).result;
    }
    const archiveId = 'archive-test';
    const audit = cli('audit');
    const archiveBody = { ...base, action: 'archive', expectedRevision: 1, requestId: archiveId, auditHash: audit.auditHash };
    await post(archiveBody, 202);
    const archived = await job(archiveId);
    assert.equal(archived.state, 'completed', JSON.stringify(archived.error));
    assert.equal(archived.result.archivedAssets, 1);
    assert.equal(archived.result.permanentlyDeleted, false);
    assert.deepEqual(fs.readFileSync(path.join(archived.result.archivePath, id + '.png')), data);
    assert.equal(fs.existsSync(path.join(moved.result.path, 'assets', id + '.png')), false);
    assert.deepEqual(cli('result', { schemaVersion: 1, requestId: archiveId }).result, archived.result);
    assert.deepEqual(legacyManager.recoverReceipt(archiveId), archived.result);

    const intent = readJsonForRecovery();
    put(f.root, 'library-archive-intent.json', { ...intent, state: 'pending' });
    const recovered = cli('audit');
    assert.equal(recovered.candidateCount, 1);
    assert.deepEqual(fs.readFileSync(path.join(moved.result.path, 'assets', id + '.png')), data);
    assert.equal(readJsonForRecovery().state, 'aborted');
    function readJsonForRecovery() {
        return JSON.parse(fs.readFileSync(path.join(f.root, 'library-archive-intent.json'), 'utf8'));
    }
});

test('native calendar exchanges existing Windows DPAPI files with the legacy service', async t => {
    const f = await fixture(t);
    const moment = new Date(Date.now() + 3600000).toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z');
    const end = new Date(Date.now() + 7200000).toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z');
    const ics = title => ['BEGIN:VCALENDAR', 'VERSION:2.0', 'BEGIN:VEVENT', 'UID:fixture',
        'DTSTART:' + moment, 'DTEND:' + end, 'SUMMARY:' + title, 'END:VEVENT', 'END:VCALENDAR'].join('\r\n');
    const call = async (action, options, status = 200, legacy = false) =>
        result(await f.call('/api/calendar/' + action, options, legacy), status);
    const name = '舊設定測試';
    const first = await call('import', { body: { name, text: ics('legacy-private-title') } }, 200, true);
    const file = root => path.join(root, 'calendar', 'connection.bin');
    fs.mkdirSync(path.dirname(file(f.root)), { recursive: true });
    fs.copyFileSync(file(f.legacy), file(f.root));
    const existing = await call('events', { wallpaper: true });
    assert.deepEqual(existing.events, first.events);
    assert.equal(existing.name, name);
    assert.equal(existing.url, undefined);
    assert.equal(existing.text, undefined);
    const oldBytes = fs.readFileSync(file(f.root));
    assert.equal((await call('import', { body: { text: 'invalid' } }, 400)).error.code, 'ICS_INVALID');
    assert.deepEqual(fs.readFileSync(file(f.root)), oldBytes);
    assert.equal((await call('import', { body: { text: ics('forbidden') }, wallpaper: true }, 403)).error.code, 'ROLE_DENIED');
    assert.equal((await call('subscribe', { body: { url: 'webcal://127.1/private' } }, 400)).error.code, 'ICS_URL_BLOCKED');
    const imported = await call('import', { body: { name, text: ics('native-private-title') } });
    assert.equal(imported.events[0].title, 'native-private-title');
    assert.equal(fs.readFileSync(file(f.root)).includes(Buffer.from('native-private-title')), false);
    fs.copyFileSync(file(f.root), file(f.legacy));
    const reread = await createCalendarService({ store: new Store(f.legacy) }).events();
    assert.deepEqual(reread.events, imported.events);
    await f.restart();
    assert.deepEqual((await call('events')).events, imported.events);
    await call('disconnect', { body: {} });
    assert.equal((await call('events')).configured, false);
});

test('native weather, feeds and focus preserve legacy status and session boundaries', async t => {
    const rssUrl = 'https://example.org/feed';
    const now = Date.now();
    const cards = { github: { enabled: false, x: 3, y: 58, fontSize: 16 }, yahoo: { enabled: false, x: 52, y: 58, fontSize: 16 } };
    const rss = { enabled: false, title: '測試 RSS', url: rssUrl, x: 52, y: 4, fontSize: 16, intervalMinutes: 60 };
    const seed = (root) => {
        fs.mkdirSync(root, { recursive: true });
        put(root, 'daily-feed-settings.json', { schemaVersion: 1, revision: 2, cards, rss });
        put(root, 'daily-feed-rss.json', { schemaVersion: 1, sourceKey: hash(rssUrl), fetchedAtMs: now,
            items: [{ title: 'Cached story', url: 'https://example.org/story', description: '', dailyStars: '', publishedAt: null }] });
    };
    const f = await fixture(t, root => {
        seed(root);
        return {
            providersService: createProvidersService({
                root, executable: path.join(path.dirname(exe), 'atmos-wallpaper.exe'), run: () => null,
                fetcher: () => { throw Error('offline fixture'); },
            }),
            dailyFeedsService: createDailyFeeds({ root, fetcher: () => { throw Error('disabled fixture fetched'); } }),
        };
    });
    seed(f.root);
    const weather = '/api/providers/weather';
    const provider = '/api/providers/settings';
    const feeds = '/api/daily-feeds';
    const request = async (route, options, expected = 200, legacy = false) => result(await f.call(route, options, legacy), expected);
    for (const endpoint of [weather, provider, feeds]) {
        assert.deepEqual(await request(endpoint), await request(endpoint, {}, 200, true));
    }
    const policy = { schemaVersion: 1, expectedRevision: 0, requestId: 'weather-set', weather: { enabled: true, city: '東京' } };
    assert.equal((await request(provider, { body: policy, wallpaper: true }, 403)).error.code, 'ROLE_DENIED');
    assert.deepEqual(await request(provider, { body: policy }), await request(provider, { body: policy }, 200, true));
    assert.deepEqual(await request(weather), await request(weather, {}, 200, true));
    assert.equal((await request(weather)).offline, true);
    assert.equal((await request(provider, { body: policy })).replayed, true);

    const nextFeed = { schemaVersion: 1, expectedRevision: 2, cards, rss: { ...rss, url: 'https://example.org/new' } };
    put(f.root, '.lock', { pid: process.pid });
    try {
        assert.equal((await request(feeds, { body: nextFeed }, 409)).error.code, 'STORE_LOCKED');
    } finally {
        fs.unlinkSync(path.join(f.root, '.lock'));
    }
    assert.deepEqual(await request(feeds, { body: nextFeed }), await request(feeds, { body: nextFeed }, 200, true));
    assert.deepEqual((await request(feeds, { wallpaper: true })).feeds.rss.items, []);
    assert.equal((await request(feeds, { body: nextFeed }, 409)).error.code, 'FEED_REVISION_CONFLICT');
    assert.equal((await request(feeds, { body: { ...nextFeed, expectedRevision: 3, rss: { ...rss, url: 'https://127.1/feed' } } }, 400)).error.code, 'FEED_SETTINGS_INVALID');

    const focus = '/api/host/focus';
    const focusUpdate = { schemaVersion: 1, expectedRevision: 0, requestId: 'focus-test', mode: 'focus' };
    assert.deepEqual(await request(focus, { body: focusUpdate }), await request(focus, { body: focusUpdate }, 200, true));
    assert.equal((await request(focus, { wallpaper: true })).mode, 'focus');
    await f.restart();
    assert.equal((await request(focus)).mode, 'normal');
});

test('native presets and schedule read legacy catalogs and replay requests in both directions', async t => {
    const f = await fixture(t);
    const presets = '/api/presets';
    const schedule = '/api/schedule';
    const call = async (route, body, legacy = false, status = 200) => result(await f.call(route, { body }, legacy), status);
    for (const route of [presets, schedule]) {
        assert.equal((await result(await f.call(route, { wallpaper: true }), 403)).error.code, 'ROLE_DENIED');
    }
    const save = { schemaVersion: 1, action: 'save', expectedCatalogRevision: 0, expectedRevision: 0, requestId: 'js-save', name: ' 夜晚專注 ' };
    const first = await call(presets, save, true);
    fs.copyFileSync(path.join(f.legacy, 'presets.json'), path.join(f.root, 'presets.json'));
    assert.deepEqual(await call(presets, save), { ...first, replayed: true });
    assert.equal((await call(presets)).presets[0].name, '夜晚專注');
    assert.equal((await call(presets, { ...save, name: 'different' }, false, 409)).error.code, 'REQUEST_CONFLICT');

    const nativeSave = { ...save, expectedCatalogRevision: 1, requestId: 'native-save', name: '第二份' };
    const second = await call(presets, nativeSave);
    fs.copyFileSync(path.join(f.root, 'presets.json'), path.join(f.legacy, 'presets.json'));
    assert.deepEqual(await call(presets, nativeSave, true), { ...second, replayed: true });
    assert.deepEqual(await call(presets), await call(presets, undefined, true));

    const changes = { ...command('manual', 0, 'coast'), changes: { visual: { background: 'coast', locale: 'en' },
        widgets: { todos: [{ id: 'private', text: 'existing', done: false }], timer: { minutes: 42 } } } };
    for (const legacy of [false, true]) await call('/api/commands', changes, legacy);
    const apply = { ...save, action: 'apply', expectedCatalogRevision: 2, expectedRevision: 1, requestId: 'native-apply', presetId: first.presetId };
    delete apply.name;
    const nativeApplied = await call(presets, apply);
    const legacyApplied = await call(presets, apply, true);
    assert.deepEqual(nativeApplied, legacyApplied);
    const desired = (await call('/api/snapshot')).desired;
    assert.equal(desired.visual.locale, 'en');
    assert.equal(desired.widgets.timer.minutes, 42);
    assert.equal(desired.widgets.todos[0].text, 'existing');
    assert.equal((await call(presets, apply)).replayed, true);

    const policy = { enabled: false, mode: 'interval', intervalMinutes: 30, presetIds: [second.presetId], entries: [], manualGraceMinutes: 30 };
    const scheduleSet = { schemaVersion: 1, expectedRevision: 0, requestId: 'schedule-js', policy };
    const jsSaved = await call(schedule, scheduleSet, true);
    fs.copyFileSync(path.join(f.legacy, 'schedule.json'), path.join(f.root, 'schedule.json'));
    assert.deepEqual(await call(schedule, scheduleSet), { ...jsSaved, replayed: true });
    const nativeSet = { ...scheduleSet, expectedRevision: 1, requestId: 'schedule-native', policy: { ...policy, intervalMinutes: 60 } };
    const nativeSaved = await call(schedule, nativeSet);
    fs.copyFileSync(path.join(f.root, 'schedule.json'), path.join(f.legacy, 'schedule.json'));
    assert.deepEqual(await call(schedule, nativeSet, true), { ...nativeSaved, replayed: true });
    assert.deepEqual(await call(schedule), await call(schedule, undefined, true));
    for (const [namespace, endpoint] of [['preset', presets], ['schedule', schedule]]) {
        const child = spawnSync(exe, ['--' + namespace + '-command', 'get', '--store', f.root], {
            encoding: 'utf8', windowsHide: true,
            env: { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option' },
        });
        assert.ifError(child.error);
        assert.equal(child.status, 0, child.stdout || child.stderr);
        assert.deepEqual(JSON.parse(child.stdout), { ...await call(endpoint), apiVersion: 1, ok: true,
            namespace, rendererAcknowledged: false, hostVisibility: 'unknown' });
    }

    const remove = { ...apply, action: 'delete', expectedRevision: 2, requestId: 'native-delete' };
    assert.equal((await call(presets, remove)).deleted, true);
    assert.equal((await call(presets, remove)).replayed, true);
    assert.equal((await call(presets)).presets.length, 1);
});

test('candidate timer rotates only its configured independent scope and deduplicates after restart', async t => {
    const f = await fixture(t);
    f.layout('independent', [left, right]);
    const read = async (route, id) => {
        for (let attempt = 0; attempt < 5; attempt++) {
            const response = await f.call(route, { headers: scope(id) });
            if (response.status !== 409) return result(response);
            assert.equal((await response.json()).error.code, 'STORE_LOCKED');
            await new Promise(resolve => setTimeout(resolve, 25));
        }
        assert.fail('Schedule store stayed locked');
    };
    const post = async (route, body, id) => result(await f.call(route, { headers: scope(id), body }));
    const before = await read('/api/presets', left);
    const saved = await post('/api/presets', { schemaVersion: 1, action: 'save', requestId: 'left-save', expectedCatalogRevision: 0, expectedRevision: before.currentRevision, name: '左' }, left);
    assert.equal((await read('/api/presets', right)).presets.length, 0);
    const now = Date.now();
    const intervalMinutes = [1440, 1439, 1438, 1437].find(minutes => minutes * 60000 - now % (minutes * 60000) > 60000);
    assert.ok(intervalMinutes, 'Test requires a stable schedule slot across restart');
    await post('/api/schedule', { schemaVersion: 1, expectedRevision: 0, requestId: 'left-timer', policy: {
        enabled: true, mode: 'interval', intervalMinutes, presetIds: [saved.presetId], entries: [], manualGraceMinutes: 30,
    } }, left);
    async function waitForSchedule() {
        for (let attempt = 0; attempt < 30; attempt++) {
            const value = (await read('/api/schedule', left)).runtime;
            if (value?.state === 'applied') return value;
            await new Promise(resolve => setTimeout(resolve, 250));
        }
        assert.fail('Native timer did not commit the configured schedule');
    }
    const status = await waitForSchedule();
    assert.equal(status.lastAppliedRevision, before.currentRevision + 1);
    assert.equal((await read('/api/presets', right)).currentRevision, before.currentRevision);
    assert.equal(JSON.parse(fs.readFileSync(path.join(f.root, 'live.json'), 'utf8')).revision, 0);
    await f.restart();
    await new Promise(resolve => setTimeout(resolve, 5200));
    assert.equal((await read('/api/presets', left)).currentRevision, status.lastAppliedRevision);
    assert.deepEqual((await read('/api/schedule', left)).runtime, status);
});

test('native control envelopes and errors match the legacy router and wrappers', async t => {
    const f = await fixture(t, root => {
        const executable = path.join(path.dirname(exe), 'atmos-wallpaper.exe');
        const run = (args, value) => {
            const child = spawnSync(exe, args, { input: value, encoding: 'utf8', windowsHide: true });
            assert.ifError(child.error);
            const output = JSON.parse(child.stdout);
            if (child.status !== 0) throw Object.assign(Error(output.error.code), { code: output.error.code });
            return output;
        };
        let enabled = false;
        return {
            displayService: createDisplayService({ executable, store: root, run: (_, args, value) => JSON.stringify(run(args, value)) }),
            layoutService: createLayoutService({ executable, store: root, run }),
            startupService: createStartupService({ executable, run: (_, [, action]) => {
                if (action !== 'status') enabled = action === 'enable';
                return JSON.stringify({ available: true, enabled, scope: 'current-user', trigger: 'logon', command: 'private' });
            } }),
            audioService: createAudioService({ executable, store: root, run: (args, value) => run(args, JSON.stringify(value)) }),
            performanceService: createPerformanceService({ executable, store: root, run }),
            libraryService: createLibraryService({ executable, root, initialize: false, run: () => ({ opened: true }) }),
            providersService: createProvidersService({ executable, root, run: () => run(['--system-info']), runDisks: () => run(['--disk-info']) }),
        };
    });
    async function compare(endpoint, body, select, status = 200) {
        const options = body === undefined ? {} : { body };
        const native = await result(await f.call(endpoint, options), status);
        const legacy = await result(await f.call(endpoint, options, true), status);
        assert.deepEqual(select(native), select(legacy), endpoint);
    }
    const settings = ({ schemaVersion, available, settings, result }) => ({ schemaVersion, available, settings, result });
    const identity = v => v;
    const rev = { schemaVersion: 1, expectedRevision: 0 };
    await compare('/api/host/displays', { ...rev, placement: { enabled: false, target: 'primary' } }, settings);
    await compare('/api/host/displays', { ...rev, placement: { enabled: false, target: 'primary' } }, identity, 409);
    await compare('/api/host/layout', { ...rev, requestId: 'layout', layout: { mode: 'single', targets: [], askOnApply: true } }, settings);
    await compare('/api/host/startup', { enabled: true }, identity);
    await compare('/api/host/audio', { ...rev, requestId: 'audio', audio: { enabled: false, muted: true, volume: 40, focusOnly: false, source: 'rain', output: 'system-default' } }, settings);
    await compare('/api/host/audio/test', { ...rev, expectedRevision: 1 }, identity, 400);
    await compare('/api/host/audio/routing', { ...rev, requestId: 'audio-routing', routing: { mode: 'all', target: null } }, settings);
    await compare('/api/host/performance', { ...rev, requestId: 'performance', performance: { fps: 5, pauseFullscreen: true, pauseBattery: false } }, settings);
    await compare('/api/host/library', undefined, ({ available, online, legacy, revision, assetCount, bytes, cleanup }) => ({ available, online, legacy, revision, assetCount, bytes, cleanup }));
    await compare('/api/providers/system', undefined, ({ available, source, gpu, power, disks, stale }) => ({ available, source, gpu, power, disks: disks.map(({ drive, available }) => ({ drive, available })), stale }));
});

test('single and span snapshots, binary artwork, ETag and route policy match legacy behavior', async t => {
    const f = await fixture(t);
    for (const [mode, targets, headers] of [['single', [], {}], ['span', [left, right, offline], scope(right, 'shared')]]) {
        f.layout(mode, targets);
        const options = { wallpaper: true, headers };
        const native = await f.call('/api/snapshot', options);
        const etag = native.headers.get('etag');
        const state = await result(native);
        const legacy = await result(await f.call('/api/snapshot', options, true));
        for (const key of ['desired','revision','desiredHash','artwork','projection','projectionId','controlMode']) {
            assert.deepEqual(state[key], legacy[key], key);
        }
        const unchanged = await f.call('/api/snapshot', { ...options, headers: { ...headers, 'If-None-Match': etag } });
        assert.equal(unchanged.status, 304);
        assert.equal(await unchanged.text(), '');
        const image = await f.call(state.artwork.url, options);
        const original = await f.call(state.artwork.url, options, true);
        assert.equal(image.headers.get('content-type'), 'image/png');
        assert.deepEqual(Buffer.from(await image.arrayBuffer()), Buffer.from(await original.arrayBuffer()));
    }
    assert.equal((await f.page('/wallpaper/' + offline)).status, 200);
    const target = await (await f.page('/manager/' + right)).text();
    assert.ok(target.includes(JSON.stringify({scopeId:'shared',targetId:right})));
    assert.equal((await result(await f.call('/api/snapshot'), 409)).error.code, 'TARGET_REQUIRED');
    assert.equal((await result(await f.call('/api/snapshot', { headers: scope(right) }), 401)).error.code, 'SCOPE_CHANGED');
    const health = await result(await f.call('/api/health'));
    assert.deepEqual(health.perTarget.map(t => t.online), [true,true,false]);
    assert.equal(health.allTargetsAcknowledged, false);
});

test('independent scopes reject invalid ownership before writing and isolate commits and receipt replay', async t => {
    const f = await fixture(t);
    f.layout('independent', [left,right]);
    assert.equal((await result(await f.call('/api/snapshot', { headers: scope(right,'shared') }), 401)).error.code,'SCOPE_CHANGED');
    assert.equal(fs.existsSync(path.join(f.root,'targets',right)),false);
    for (const reference of [false,true]) {
        for (const id of [left,right]) {
            const snapshot = await result(await f.call('/api/snapshot',{headers:scope(id)},reference));
            assert.equal(snapshot.revision,1);
        }
    }
    const body = command('same-request',1,'city');
    for (const reference of [false,true]) {
        const commit=await result(await f.call('/api/commands',{body,headers:scope(left)},reference));
        const replay=await result(await f.call('/api/commands',{body,headers:scope(left)},reference));
        assert.equal(commit.result.revision,2);
        assert.equal(replay.result.replayed,true);
        const other=await result(await f.call('/api/snapshot',{headers:scope(right)},reference));
        assert.equal(other.revision,1);
        assert.notEqual(other.desired.visual.background,'city');
    }
    const marker=read(f.root,'targets/'+right+'/scope.json');
    put(f.root,'targets/'+right+'/scope.json',{...marker,ownerKey:'wrong-owner'});
    assert.equal((await result(await f.call('/api/snapshot',{headers:scope(right)}),503)).error.code,'SCOPE_OWNER_MISMATCH');
});

test('preview, cancel, rollback, backup import and damaged live recovery retain CAS', async t => {
    const f=await fixture(t);
    const post=async (op,body,status=200) => result(await f.call('/api/core/'+op,{body}),status);
    const preview=await post('preview',{requestId:'draft',expectedRevision:0,patch:{visual:{background:'city'}}});
    const same=f.bridge.registry.owner.preview({requestId:'draft',expectedRevision:0,patch:{visual:{background:'city'}}});
    assert.deepEqual(preview.result,same);
    const previewId=preview.result.previewId;
    await post('cancel',{requestId:'discard',previewId});
    assert.equal((await post('apply',{requestId:'cancelled-apply',expectedRevision:0,previewId},400)).error.code,'PREVIEW_CANCELLED');
    const outcomes=await Promise.all([f.call('/api/commands',{body:command('one',0,'city')}),f.call('/api/commands',{body:command('two',0,'coast')})]);
    assert.equal(outcomes.filter(r=>r.status===200).length,1);
    assert.ok(outcomes.every(r=>[200,409].includes(r.status)));
    await Promise.all(outcomes.map(r=>r.text()));
    const backup=await post('backup',{});
    const { backupHash }=backup.result;
    const backupFile='backups/'+backupHash+'.json';
    const bundle=read(f.root,backupFile);
    f.bridge.registry.owner.restoreBackup({requestId:'legacy-check',expectedRevision:0,backup:bundle});
    await post('rollback',{requestId:'undo',expectedRevision:1,targetRevision:0});
    put(f.root,'imports/'+backupHash+'.json',{...bundle,desiredHash:'wrong'});
    assert.equal((await post('import',{requestId:'tampered',expectedRevision:2,backupHash},400)).error.code,'BACKUP_CORRUPT');
    put(f.root,'imports/'+backupHash+'.json',bundle);
    const restored=await post('import',{requestId:'from-backup',expectedRevision:2,backupHash});
    assert.equal(restored.result.restorePreview,true);
    assert.equal((await post('apply',{requestId:'commit-restore',expectedRevision:2,previewId:restored.result.previewId})).result.revision,3);
    fs.writeFileSync(path.join(f.root,'live.json'),'{corrupt');
    await f.restart();
    assert.equal((await result(await f.call('/api/snapshot'),503)).error.code,'CORRUPT_STORE');
    const recovered=await post('recover',{requestId:'recover'});
    assert.equal(recovered.result.recovered,true);
    assert.equal(recovered.result.recoveredFromRevision,2);
    assert.equal(recovered.result.revision,4);
    assert.equal((await post('recover',{requestId:'recover'})).result.replayed,true);
    assert.equal((await post('apply',{requestId:'commit-restore',expectedRevision:2,previewId:restored.result.previewId},409)).error.code,'RECOVERED_OUTCOME_UNKNOWN');
});

test('isolated report validation rejects stale projections and separates manager and wallpaper reports', async t => {
    const f=await fixture(t);
    f.layout('duplicate',[left,right]);
    const headers=scope(left,'shared');
    const snap=await result(await f.call('/api/snapshot',{headers}));
    const syntheticReport={schemaVersion:1,rendererId:'fixture-renderer',renderedAt:new Date().toISOString(),stateApplied:true,
        revision:snap.revision,desiredHash:snap.desiredHash,assetSha256:snap.artwork.sha256,projectionId:snap.projectionId};
    const stale=await result(await f.call('/api/ack',{wallpaper:true,headers:scope(right,'shared'),body:syntheticReport}),409);
    assert.equal(stale.error.code,'ACK_STALE');
    await result(await f.call('/api/ack',{headers,body:syntheticReport}));
    let health=await result(await f.call('/api/health'));
    assert.equal(health.renderers[0].role,'manager');
    assert.equal(health.allTargetsAcknowledged,false);
    await result(await f.call('/api/ack',{wallpaper:true,headers,body:syntheticReport}));
    health=await result(await f.call('/api/health'));
    assert.deepEqual(health.perTarget.map(t=>t.rendererAcknowledged),[true,false]);
    assert.equal(health.nativeAllAttached,false);
    assert.equal(health.hostVisibility,'unknown');
    await f.restart();
    health=await result(await f.call('/api/health'));
    assert.deepEqual(health.perTarget.map(t=>t.rendererAcknowledged),[false,false]);
    await result(await f.call('/api/ack',{wallpaper:true,headers,body:{...syntheticReport,renderedAt:new Date().toISOString()}}));
    f.layout('span',[left,right],2);
    health=await result(await f.call('/api/health'));
    assert.deepEqual(health.perTarget.map(t=>t.rendererAcknowledged),[false,false]);
});

test('custom backup bytes are normalized before import and offline export matches the legacy renderer', async t => {
    const f = await fixture(t);
    const owner = f.bridge.registry.owner;
    const image=spawnSync(python,['-B','-c','import sys; from PIL import Image; Image.new("RGB", (5, 7), (20, 40, 60)).save(sys.stdout.buffer, format="PNG")'],{windowsHide:true,timeout:10000});
    assert.equal(image.status,0,String(image.stderr));
    fs.writeFileSync(path.join(f.legacy,'imports/art.png'),image.stdout);
    const { asset } = owner.stage({ requestId: 'legacy-stage', file: 'art.png' });
    const draft = owner.preview({ requestId: 'legacy-draft', expectedRevision: 0, patch: { assetId: asset.assetId } });
    owner.apply({ requestId: 'legacy-apply', expectedRevision: 0, previewId: draft.previewId });
    const legacyBackup = owner.backup();
    const backup = read(f.legacy, 'backups/'+legacyBackup.backupHash+'.json');
    const broken = structuredClone(backup);
    delete broken.backupHash;
    broken.assets[0].width++;
    broken.backupHash = hash(broken);
    put(f.root, 'imports/'+broken.backupHash+'.json', broken);
    const bad = await result(await f.call('/api/core/import', { body: {
        requestId:'bad-image', expectedRevision:0, backupHash:broken.backupHash,
    }}),400);
    assert.equal(bad.error.code,'IMAGE_REJECTED');
    assert.equal(fs.readdirSync(path.join(f.root,'assets')).length,0);
    fs.writeFileSync(path.join(f.root,'imports/art.png'),image.stdout);
    const stageInput = { requestId: 'native-stage', file: 'art.png' };
    assert.equal((await result(await f.call('/api/core/stage', { body: { ...stageInput, file: '../outside.png' } }), 400)).error.code, 'PATH_REJECTED');
    const nativeStage = await result(await f.call('/api/core/stage', { body: stageInput }));
    assert.deepEqual(nativeStage.result.asset, asset);
    assert.equal(nativeStage.result.liveUnchanged, true);
    assert.equal((await result(await f.call('/api/core/stage', { body: stageInput }))).result.replayed, true);
    put(f.root,'imports/'+backup.backupHash+'.json',backup);
    const staged=await result(await f.call('/api/core/import',{body:{requestId:'valid-image',expectedRevision:0,backupHash:backup.backupHash}}));
    assert.equal(staged.result.liveUnchanged,true);
    await result(await f.call('/api/core/apply',{body:{requestId:'apply-image',expectedRevision:0,previewId:staged.result.previewId}}));
    const current=await result(await f.call('/api/snapshot',{wallpaper:true}));
    assert.equal(current.artwork.sha256,asset.assetId);
    const artwork=await f.call(current.artwork.url,{wallpaper:true});
    assert.deepEqual(Buffer.from(await artwork.arrayBuffer()),Buffer.from(backup.assets[0].base64,'base64'));
    const newBackup=await result(await f.call('/api/core/backup',{body:{}}));
    assert.equal(newBackup.result.backupHash,legacyBackup.backupHash);
    const restored=await result(await f.call('/api/core/restore',{body:{requestId:'local-backup',expectedRevision:1,backupHash:backup.backupHash}}));
    assert.equal(restored.result.candidateHash,current.desiredHash);
    const candidate=(await result(await f.call('/api/core/export',{body:{previewId:restored.result.previewId}}))).result;
    assert.equal(candidate.snapshot.kind,'candidate-preview');
    const exported=(await result(await f.call('/api/core/export',{body:{}}))).result;
    const legacy=exportRenderer(owner);
    const withoutId=({bundleId,...snapshot})=>snapshot;
    assert.deepEqual(withoutId(exported.snapshot),withoutId(legacy.snapshot));
    const manifest=read(exported.directory,'p1-manifest.json');
    const legacyManifest=read(legacy.directory,'p1-manifest.json');
    assert.deepEqual(manifest.files.map(f=>f.path).sort(),legacyManifest.files.map(f=>f.path).sort());
    for(const file of manifest.files) {
        const bytes=fs.readFileSync(path.join(exported.directory,file.path));
        assert.equal(hash(bytes),file.sha256);
        if(file.path!=='renderer-state.js') assert.deepEqual(bytes,fs.readFileSync(path.join(legacy.directory,file.path)),file.path);
    }
    assert.equal((await result(await f.call('/api/core/export',{body:{}}))).result.existing,true);
    const report={schemaVersion:1,bundleId:exported.bundleId,revision:current.revision,desiredHash:current.desiredHash,
        source:'preview',stateUnchanged:true,assetSha256:current.artwork.sha256,assetLoaded:true,hostVisibility:'unknown',
        capturedAt:new Date().toISOString()};
    assert.equal((await result(await f.call('/api/core/observe',{body:report}))).result.accepted,true);
    assert.equal((await result(await f.call('/api/health'))).allTargetsAcknowledged,false);
    fs.appendFileSync(exported.entry,'<!-- corrupt fixture -->');
    assert.equal((await result(await f.call('/api/core/export',{body:{}}),503)).error.code,'EXPORT_CORRUPT');
});
