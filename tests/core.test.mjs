import test from 'node:test';
import assert from 'node:assert/strict';

import {
  applyHistoryClear,
  batchStates,
  buildPublicUrl,
  canCopyAsset,
  deriveBatchState,
  renderCopyText,
  renderRoutePath,
  resolveCompressionPlan,
  shouldKeepCompressedOutput,
  visibleHistory,
} from '../src/core/index.js';

test('Smart Compress keeps original JPEG format and uses MozJPEG by default', () => {
  const plan = resolveCompressionPlan({ name: 'hero.jpg', mime: 'image/jpeg' });

  assert.equal(plan.supported, true);
  assert.equal(plan.sourceFormat, 'jpeg');
  assert.equal(plan.targetFormat, 'jpeg');
  assert.equal(plan.encoder, 'mozjpeg');
  assert.deepEqual(plan.steps, ['mozjpeg']);
});

test('Extreme AVIF uses libavif and AOM for static images', () => {
  const plan = resolveCompressionPlan({ name: 'hero.png', mime: 'image/png' }, { mode: 'extreme_avif' });

  assert.equal(plan.targetFormat, 'avif');
  assert.equal(plan.quality, 48);
  assert.equal(plan.encoder, 'libavif+aom');
  assert.deepEqual(plan.steps, ['libavif', 'aom']);
  assert.equal(plan.warnings.includes('avif_slower_encoding'), true);
});

test('Compression mode presets provide default output format and quality', () => {
  const tiny = resolveCompressionPlan({ name: 'hero.jpg', mime: 'image/jpeg' }, { mode: 'tiny' });
  const lossless = resolveCompressionPlan({ name: 'diagram.png', mime: 'image/png' }, { mode: 'lossless' });

  assert.equal(tiny.quality, 66);
  assert.equal(tiny.targetFormat, 'webp');
  assert.equal(lossless.quality, 100);
  assert.equal(lossless.targetFormat, 'png');
});

test('Animated GIF in AVIF mode falls back to WebP plan', () => {
  const plan = resolveCompressionPlan({ name: 'loop.gif', animated: true }, { outputFormat: 'avif' });

  assert.equal(plan.targetFormat, 'webp');
  assert.equal(plan.encoder, 'libwebp');
});

test('Compression result falls back when output is larger', () => {
  assert.equal(shouldKeepCompressedOutput({ sourceSize: 1000, outputSize: 900 }), true);
  assert.equal(shouldKeepCompressedOutput({ sourceSize: 1000, outputSize: 1100 }), false);
});

test('Batch state and copy rules match product flow', () => {
  const assets = [
    { id: '1', compressProgress: 100, uploadProgress: 100, publicUrl: 'https://cdn.test/a.webp' },
    { id: '2', compressProgress: 100, uploadProgress: 0 },
  ];

  assert.equal(deriveBatchState(assets), batchStates.compressed);
  assert.equal(canCopyAsset(assets[0]), true);
  assert.equal(canCopyAsset(assets[1]), false);
});

test('Copy templates render URL, Markdown, HTML and custom format', () => {
  const asset = {
    name: 'hero cover',
    publicUrl: 'https://cdn.test/hero.webp',
    width: 1200,
    height: 800,
  };

  assert.equal(renderCopyText(asset, 'url'), 'https://cdn.test/hero.webp');
  assert.equal(renderCopyText(asset, 'markdown'), '![hero cover](https://cdn.test/hero.webp)');
  assert.equal(
    renderCopyText(asset, 'html'),
    '<img src="https://cdn.test/hero.webp" alt="hero cover" width="1200" height="800" />',
  );
  assert.equal(
    renderCopyText(asset, 'custom', '<Image src="{url}" alt="{alt}" />'),
    '<Image src="https://cdn.test/hero.webp" alt="hero cover" />',
  );
});

test('Route path template produces stable CDN key and URL', () => {
  const remoteKey = renderRoutePath('{yyyy}/{mm}/{hash}-{slug}.{ext}', {
    id: 'abcdef1234567890',
    name: 'Hero Cover.JPG',
    outputFormat: 'jpeg',
  }, new Date('2026-10-11T00:00:00+08:00'));

  assert.equal(remoteKey, '2026/10/abcdef123456-hero-cover.jpg');
  assert.equal(buildPublicUrl('https://cdn.picdog.app/', remoteKey), 'https://cdn.picdog.app/2026/10/abcdef123456-hero-cover.jpg');
});

test('History clear hides records without deleting data', () => {
  const records = [{ id: 'a' }, { id: 'b' }];
  const cleared = applyHistoryClear(records, { scope: 'selected_history', ids: ['a'], clearedAt: '2026-10-11T00:00:00.000Z' });

  assert.deepEqual(visibleHistory(cleared), [{ id: 'b' }]);
  assert.equal(cleared[0].hidden, true);
  assert.equal(cleared[0].clearedAt, '2026-10-11T00:00:00.000Z');
});

