// Smoke-test a standalone native-runtime build with a disposable payload.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const host = fileURLToPath(new URL('../', import.meta.url));
const source = path.resolve(host, '..');
const candidate = process.env.ATMOS_NATIVE_CANDIDATE || path.join(host, 'target/debug/atmos-wallpaper.exe');
const python = process.env.ATMOS_EMBEDDED_PYTHON_DIR || path.join(host, 'payload/runtime/python');
const temp = fs.realpathSync(os.tmpdir());
const root = fs.mkdtempSync(path.join(temp, 'atmos-native-installcheck-'));
const payload = path.join(root, 'payload');
const exe = path.join(root, 'atmos-wallpaper.exe');
const files = [];
const staticIndex = 'app/implementation-p4_2_5/dist/index.html';

function put(relative, bytes) {
    const target = path.join(payload, relative);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, bytes);
    files.push({ path: relative, bytes: Buffer.byteLength(bytes), sha256: createHash('sha256').update(bytes).digest('hex') });
}

function copyTree(dir, relative) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const input = path.join(dir, entry.name);
        const output = relative + '/' + entry.name;
        if (entry.isDirectory()) copyTree(input, output);
        else if (entry.isFile()) put(output, fs.readFileSync(input));
        else assert.fail('Unexpected entry in the staged Python runtime');
    }
}

try {
    fs.copyFileSync(candidate, exe);
    copyTree(python, 'runtime/python');
    put(staticIndex, fs.readFileSync(path.join(source, 'implementation-p4_2_5/dist/index.html')));
    put('app/implementation-p1/src/decode-image.py', fs.readFileSync(path.join(source, 'implementation-p1/src/decode-image.py')));
    put('app/prototype-v8.1-frozen/manifest.json', fs.readFileSync(path.join(source, 'prototype-v8.1-frozen/manifest.json')));
    fs.writeFileSync(path.join(payload, 'payload-manifest.json'), JSON.stringify({
        schemaVersion: 1, architecture: 'x86_64', versions: { python: 'embedded', pillow: 'embedded' }, files,
    }));
    const env = { ...process.env, PATH: '', NODE_OPTIONS: '--invalid-fixture-option', LOCALAPPDATA: path.join(root, 'local') };
    const run = (...args) => {
        const result = spawnSync(exe, args, { env, encoding: 'utf8', timeout: 12000, windowsHide: true });
        if (result.error) throw result.error;
        return result;
    };
    const check = run('--check-install');
    assert.equal(check.status, 0, check.stderr);
    assert.equal(JSON.parse(check.stdout).payloadVerified, true);
    const library = run('--library-command', 'get', '--store', path.join(root, 'store'));
    assert.equal(library.status, 0, library.stderr);
    assert.equal(JSON.parse(library.stdout).result.assetCount, 0);
    const obsolete = ['runtime/node/node.exe', 'app/implementation-p4_2_5/node_modules/old/index.js',
        'app/implementation-p4_2_5/src/windows-runtime.mjs', 'app/implementation-p4_2_1/src/legacy.mjs'];
    for (const relative of obsolete) {
        const target = path.join(payload, relative);
        fs.mkdirSync(path.dirname(target), { recursive: true });
        fs.writeFileSync(target, 'legacy payload');
    }
    for (let attempt = 0; attempt < 2; attempt++) {
        const retire = run('--retire-node-payload');
        assert.equal(retire.status, 0, retire.stderr);
        for (const relative of obsolete) assert.equal(fs.existsSync(path.join(payload, relative)), false);
        assert.equal(run('--check-install').status, 0);
    }
    fs.appendFileSync(path.join(payload, staticIndex), 'altered');
    const corrupt = run('--check-install');
    assert.equal(corrupt.status, 1, corrupt.stderr);
    assert.match(corrupt.stderr, /Bundled file size mismatch/);
    console.log('Native CLI and install check passed without Node. Legacy payload retired and corrupt payload rejected.');
} finally {
    const resolved = fs.realpathSync(root);
    assert.equal(path.dirname(resolved), temp);
    fs.rmSync(resolved, { recursive: true, force: true });
}
