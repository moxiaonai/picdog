const assert = require('assert');
const fs = require('fs');
const path = require('path');
const core = require('../src/core/prototype-core.js');

assert.strictEqual(core.normalizeUploadEnabled(null), false);
assert.strictEqual(core.normalizeUploadEnabled('false'), false);
assert.strictEqual(core.normalizeUploadEnabled('true'), true);
assert.strictEqual(core.normalizeUploadEnabled(true), true);

const initialFiles = [
  { name: 'hero-cover.png', meta: 'PNG', saved: '−76%' },
  { name: 'product-shot.jpg', meta: 'JPEG', saved: '−77%' }
];

const files = core.cloneFiles(initialFiles);
assert.strictEqual(files.length, 2);
assert.strictEqual(core.selectedCount(files), 0);
assert.strictEqual(core.uploadedCount(files), 0);

files[0].selected = true;
assert.deepStrictEqual(core.selectedUploadIndexes(files), [0]);

core.setAllCompression(files, 100);
let history = core.upsertHistoryFromFiles([], files, 'Blog CDN · R2', 8);
assert.strictEqual(history.length, 2);
assert.strictEqual(history[0].statusKey, 'historyCompressed');
assert.strictEqual(history[0].url, '');

core.setUploadProgress(files, [0, 1], 100);
assert.strictEqual(core.nextStateAfterUpload(files), 'uploaded');
history = core.upsertHistoryFromFiles(history, files, 'Blog CDN · R2', 8);
assert.strictEqual(history.length, 2);
assert.strictEqual(history[0].statusKey, 'historyUploaded');
assert.ok(history[0].url.includes('https://cdn.picdog.app/2026/10/'));

const root = path.resolve(__dirname, '..');
const html = fs.readFileSync(path.join(root, 'index.html'), 'utf8');
const app = fs.readFileSync(path.join(root, 'app.js'), 'utf8');
const css = fs.readFileSync(path.join(root, 'styles.css'), 'utf8');
const switchIndex = html.indexOf('id="uploadEnabledSwitch"');
const targetIndex = html.indexOf('id="targetSelect"');

assert.ok(switchIndex >= 0, 'CDN settings should include an upload switch');
assert.ok(switchIndex < targetIndex, 'upload switch should be the first CDN setting');
assert.ok(html.includes('role="switch"'), 'upload preference should expose switch semantics');
assert.ok(html.includes('settings-row upload-only-setting'), 'upload-only copy settings should be marked for strict disabled mode');
assert.ok(app.includes("const UPLOAD_ENABLED_STORAGE_KEY = 'picdog.uploadEnabled'"), 'upload preference should be persisted');
assert.ok(app.includes('if (!uploadEnabled) return;'), 'upload actions should guard the disabled state');
assert.ok(app.includes("const copyMarkup = uploadEnabled"), 'history copy actions should only render when upload is enabled');
assert.ok(css.includes('.app-panel[data-upload-enabled="false"] .progress-line'), 'local-only rows should use a compact progress layout');
assert.ok(css.includes('.app-panel[data-upload-enabled="false"] .upload-only-setting'), 'copy format should be hidden while upload is disabled');

console.log('PicDog core smoke test passed');
