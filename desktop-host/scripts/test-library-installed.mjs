// Exercise an installed layout using only the native host and bundled decoder.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';

const args = process.argv.slice(2);
if (process.platform !== 'win32' || (args.length && (args.length !== 2 || args[0] !== '--install-root'))) {
    throw Error('Windows only. Usage: test-library-installed.mjs [--install-root DIRECTORY]');
}
const install = fs.realpathSync(args[1] || path.join(process.env.LOCALAPPDATA, 'Atmos Wallpaper'));
const exe = path.join(install, 'atmos-wallpaper.exe');
const payload = path.join(install, 'payload');
const app = path.join(payload, 'app');
const python = path.join(payload, 'runtime/python/python.exe');
const decoder = path.join(app, 'implementation-p1/src/decode-image.py');
const temp = fs.realpathSync(os.tmpdir());
const scratch = fs.mkdtempSync(path.join(temp, 'atmos-library-installed-'));
const store = path.join(scratch, 'store');
const target = path.join(scratch, '中文 空白素材庫');
const env = { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option', LOCALAPPDATA: path.join(scratch, 'local') };
delete env.PYTHONHOME;
delete env.PYTHONPATH;

function run(file, args, input) {
    const result = spawnSync(file, args, { encoding: 'utf8', input, env, windowsHide: true, timeout: 30000 });
    if (result.error) throw result.error;
    return result;
}

function command(action, input) {
    return run(exe, ['--library-command', action, '--store', store], input && JSON.stringify(input));
}

function cli(action, input) {
    const result = command(action, input);
    assert.equal(result.status, 0, result.stderr || result.stdout);
    const response = JSON.parse(result.stdout);
    assert.equal(response.ok, true);
    return response.result;
}

const checks = [];
let worker, closed, lines, readinessTimeout;
try {
    const check = run(exe, ['--check-install']);
    assert.equal(check.status, 0, check.stderr);
    const verified = JSON.parse(check.stdout);
    assert.equal(verified.payloadVerified, true);
    assert.equal(verified.embeddedRuntimesVerified, true);
    assert.equal('node' in verified.versions, false);
    for (const relative of ['runtime/node', 'app/implementation-p4_2_5/node_modules']) {
        assert.equal(fs.existsSync(path.join(payload, relative)), false, relative);
    }

    worker = spawn(exe, [
        '--native-runtime-worker', store, path.join(app, 'implementation-p4_2_5/dist'),
        path.join(app, 'prototype-v8.1-frozen'), python, decoder, process.pid + '-' + Date.now(),
    ], { env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
    closed = once(worker, 'close');
    closed.catch(() => {});
    lines = createInterface({ input: worker.stdout });
    let stderr = '';
    worker.stderr.setEncoding('utf8').on('data', chunk => { stderr = (stderr + chunk).slice(-4096); });
    const ready = once(lines, 'line');
    worker.stdin.on('error', () => {});
    worker.stdin.write('start\n');
    const [line] = await Promise.race([
        ready,
        closed.then(([code]) => { throw Error('Native worker exited ' + code + ': ' + stderr); }),
        new Promise((_, reject) => { readinessTimeout = setTimeout(() => reject(Error('Native worker readiness timeout')), 12000); }),
    ]);
    clearTimeout(readinessTimeout);
    const { origin } = JSON.parse(line);
    assert.match(origin, /^http:\/\/127\.0\.0\.1:\d+$/);
    const token = await new Promise((resolve, reject) => {
        const request = http.get(origin + '/manager', {
            headers: { 'Sec-Fetch-Mode': 'navigate', 'Sec-Fetch-Dest': 'document', 'Sec-Fetch-Site': 'none' },
            signal: AbortSignal.timeout(10000),
        }, response => {
            let html = '';
            response.setEncoding('utf8').on('data', chunk => { html += chunk; });
            response.on('error', reject);
            response.on('end', () => {
                if (response.statusCode !== 200) return reject(Error('Manager HTTP ' + response.statusCode));
                resolve(html.match(/window\.atmosBridgeSession="([a-f0-9]{64})"/)?.[1]);
            });
        });
        request.on('error', reject);
    });
    assert.ok(token);
    const auth = { 'X-Atmos-Session': token, 'Sec-Fetch-Site': 'same-origin' };
    async function request(route, body) {
        const response = await fetch(origin + route, {
            headers: body === undefined ? auth : { ...auth, Origin: origin, 'Content-Type': 'application/json' },
            ...(body === undefined ? {} : { method: 'POST', body: JSON.stringify(body) }),
            signal: AbortSignal.timeout(10000),
        });
        assert.equal(response.status, 200, route);
        return response;
    }
    async function core(action, body) {
        const response = await (await request('/api/core/' + action, body)).json();
        assert.equal(response.ok, true, action);
        return response.result;
    }

    const imports = path.join(store, 'imports');
    const image = path.join(imports, 'test.png');
    const generated = run(python, ['-I', '-B', '-c',
        "from PIL import Image;import sys;Image.new('RGB',(32,18),(14,80,130)).save(sys.argv[1],'PNG')", image]);
    assert.equal(generated.status, 0, generated.stderr);
    const { asset } = await core('stage', { requestId: 'native.stage', file: 'test.png' });
    const { previewId } = await core('preview', { requestId: 'native.preview', expectedRevision: 0, patch: { assetId: asset.assetId } });
    await core('apply', { requestId: 'native.apply', expectedRevision: 0, previewId });
    const backup = await core('backup', {});
    const appearance = fs.readFileSync(path.join(store, 'live.json'));

    const move = { schemaVersion: 1, requestId: 'native.move.001', expectedRevision: 0, path: target };
    assert.equal(cli('move', move).committed, true);
    assert.equal(cli('get').assetCount, 1);
    assert.ok(fs.existsSync(path.join(store, 'assets', asset.assetId + '.png')));
    assert.equal(cli('move', move).replayed, true);
    assert.equal(cli('result', { schemaVersion: 1, requestId: move.requestId }).result.committed, true);
    checks.push('native-stage-backup-move-replay-receipt');

    const artwork = await request('/api/artwork/' + asset.assetId);
    assert.equal(createHash('sha256').update(Buffer.from(await artwork.arrayBuffer())).digest('hex'), asset.assetId);
    assert.equal((await core('backup', {})).backupHash, backup.backupHash);
    assert.deepEqual(fs.readFileSync(path.join(store, 'live.json')), appearance);
    checks.push('asset-id-settings-and-backup-preserved-after-move');

    fs.renameSync(target, target + '.offline');
    assert.equal(cli('get').online, false);
    assert.equal(cli('get').path, target);
    assert.ok(!fs.existsSync(target));
    fs.renameSync(target + '.offline', target);
    assert.equal(cli('get').online, true);
    checks.push('real-filesystem-offline-reconnect-no-recreation');

    const stale = command('reset', { schemaVersion: 1, requestId: 'native.stale', expectedRevision: 0 });
    assert.equal(stale.status, 2);
    assert.equal(JSON.parse(stale.stdout).error.code, 'LIBRARY_REVISION_CONFLICT');
    checks.push('native-cas-conflict');

    const who = run(path.join(process.env.SystemRoot, 'System32/whoami.exe'), ['/user', '/fo', 'csv', '/nh']);
    assert.equal(who.status, 0, who.stderr);
    const sid = who.stdout.match(/S-1-[0-9-]+/)?.[0];
    assert.ok(sid);
    const denied = path.join(scratch, 'denied-target');
    fs.mkdirSync(denied);
    const icacls = path.join(process.env.SystemRoot, 'System32/icacls.exe');
    assert.equal(run(icacls, [denied, '/deny', '*' + sid + ':(OI)(CI)(W)']).status, 0);
    try {
        const failed = command('move', { schemaVersion: 1, requestId: 'native.denied', expectedRevision: 1, path: denied });
        assert.notEqual(failed.status, 0);
        assert.equal(cli('get').path, target);
        assert.ok(fs.existsSync(path.join(target, 'assets', asset.assetId + '.png')));
    } finally {
        assert.equal(run(icacls, [denied, '/remove:d', '*' + sid]).status, 0);
    }
    checks.push('actual-windows-acl-write-denial-preserves-active-library');

    assert.equal(cli('reset', { schemaVersion: 1, requestId: 'native.default', expectedRevision: 1 }).committed, true);
    assert.equal(cli('get').isDefault, true);
    assert.ok(fs.existsSync(path.join(target, 'assets', asset.assetId + '.png')));
    checks.push('reset-to-dedicated-default-retains-source');

    for (const badPath of [path.join(install, 'should-not-write'), '\\\\server\\share\\library']) {
        const failed = command('move', { schemaVersion: 1, requestId: 'native.unsafe', expectedRevision: 2, path: badPath });
        assert.notEqual(failed.status, 0);
        assert.equal(cli('get').isDefault, true);
    }
    checks.push('install-directory-and-network-path-rejected');

    const library = await (await request('/api/host/library')).json();
    assert.equal(library.available, true);
    assert.equal(library.online, true);
    checks.push('authenticated-native-library-api');
    console.log(JSON.stringify({ passed: true, checks }));
} finally {
    clearTimeout(readinessTimeout);
    if (worker) {
        worker.stdin.end();
        const stop = setTimeout(() => worker.kill(), 3000);
        await closed.catch(() => {});
        clearTimeout(stop);
        lines?.close();
    }
    assert.equal(path.dirname(fs.realpathSync(scratch)), temp);
    fs.rmSync(scratch, { recursive: true, force: true });
}
