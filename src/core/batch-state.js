export const batchStates = Object.freeze({
  empty: 'empty',
  queue: 'queue',
  compressing: 'compressing',
  compressed: 'compressed',
  uploading: 'uploading',
  uploaded: 'uploaded',
  failed: 'failed',
});

export function canCopyAsset(asset) {
  return Boolean(asset && asset.uploadProgress === 100 && asset.publicUrl);
}

export function canUploadAsset(asset) {
  return Boolean(asset && asset.compressProgress === 100 && asset.uploadProgress < 100);
}

export function deriveBatchState(assets = []) {
  if (!assets.length) return batchStates.empty;

  const hasCompressing = assets.some((asset) => asset.compressProgress > 0 && asset.compressProgress < 100);
  if (hasCompressing) return batchStates.compressing;

  const allCompressed = assets.every((asset) => asset.compressProgress === 100);
  const hasUploading = assets.some((asset) => asset.uploadProgress > 0 && asset.uploadProgress < 100);
  if (hasUploading) return batchStates.uploading;

  const allUploaded = assets.every((asset) => asset.uploadProgress === 100);
  if (allCompressed && allUploaded) return batchStates.uploaded;
  if (allCompressed) return batchStates.compressed;

  return batchStates.queue;
}

export function getBatchActions(batchState, assets = []) {
  const selected = assets.filter((asset) => asset.selected);
  const uploadableSelected = selected.filter(canUploadAsset);
  const hasFiles = assets.length > 0;

  return {
    canChooseFiles: batchState === batchStates.empty,
    canCompress: batchState === batchStates.queue && hasFiles,
    canClear: hasFiles && ![batchStates.compressing, batchStates.uploading].includes(batchState),
    canSelectAll: [batchStates.compressed, batchStates.uploaded].includes(batchState),
    canUploadSelected:
      [batchStates.compressed, batchStates.uploaded].includes(batchState) && uploadableSelected.length > 0,
    selectedCount: selected.length,
    uploadableSelectedCount: uploadableSelected.length,
  };
}

export function toAssetViewModel(asset) {
  return {
    ...asset,
    canUpload: canUploadAsset(asset),
    canCopy: canCopyAsset(asset),
    isDone: asset.compressProgress === 100 && asset.uploadProgress === 100,
  };
}

