import type { AssetState, BatchState } from '../../contracts/models';

export function canUpload(asset: AssetState) {
  return asset.compressStatus === 'done' &&
    Boolean(asset.outputPath) &&
    asset.uploadStatus !== 'done' &&
    asset.uploadStatus !== 'running';
}

export function canCopy(asset: AssetState) {
  return asset.uploadStatus === 'done' && Boolean(asset.publicUrl);
}

export function canReveal(asset: AssetState) {
  return asset.compressStatus === 'done' && Boolean(asset.outputPath);
}

export function getBatchState(assets: AssetState[]): BatchState {
  if (assets.length === 0) return 'empty';
  if (assets.some((asset) => asset.compressStatus === 'running')) return 'compressing';
  if (assets.some((asset) => asset.uploadStatus === 'running')) return 'uploading';

  const allTerminal = assets.every((asset) =>
    ['failed', 'cancelled'].includes(asset.compressStatus) ||
    asset.uploadStatus === 'done'
  );

  if (allTerminal && assets.some((asset) => asset.uploadStatus === 'done')) {
    return 'uploaded';
  }

  if (allTerminal) return 'failed';

  if (assets.some((asset) => asset.compressStatus === 'done')) {
    const uploadable = assets.filter(canUpload);

    if (uploadable.length > 0 && uploadable.every((asset) => asset.uploadStatus === 'done')) {
      return 'uploaded';
    }

    return 'compressed';
  }

  return 'queue';
}

export function formatBytes(bytes: number) {
  if (bytes < 1024) return bytes + ' B';

  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = units.shift();

  while (value >= 1024 && units.length > 0) {
    value /= 1024;
    unit = units.shift();
  }

  return value.toFixed(value >= 10 ? 1 : 2) + ' ' + unit;
}
