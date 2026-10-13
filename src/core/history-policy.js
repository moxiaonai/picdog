export const clearScopes = Object.freeze({
  currentBatch: 'current_batch',
  allHistory: 'all_history',
  selectedHistory: 'selected_history',
});

export function createHistoryItemFromAsset(asset, batch = {}) {
  const sourceSize = Number(asset.sourceSize || 0);
  const outputSize = asset.outputSize == null ? undefined : Number(asset.outputSize);
  const savedRatio =
    sourceSize > 0 && outputSize != null ? Math.max(0, Math.round((1 - outputSize / sourceSize) * 100)) : undefined;

  return {
    id: asset.id,
    batchId: batch.id || asset.batchId,
    fileName: asset.name || asset.fileName,
    sourceSize,
    outputSize,
    savedRatio,
    outputFormat: asset.outputFormat || asset.targetFormat,
    routeName: batch.routeName || asset.routeName,
    publicUrl: asset.publicUrl || '',
    compressStatus: asset.compressProgress === 100 ? 'done' : asset.error ? 'failed' : 'pending',
    uploadStatus: asset.uploadProgress === 100 ? 'done' : asset.uploadProgress > 0 ? 'pending' : 'none',
    canCopy: Boolean(asset.publicUrl && asset.uploadProgress === 100),
    createdAt: batch.createdAt || new Date().toISOString(),
    uploadedAt: asset.uploadedAt,
    copiedAt: asset.copiedAt,
  };
}

export function applyHistoryClear(records, input = {}) {
  const scope = input.scope || clearScopes.allHistory;
  const now = input.clearedAt || new Date().toISOString();

  if (scope === clearScopes.allHistory) {
    return records.map((record) => ({ ...record, hidden: true, clearedAt: now }));
  }

  if (scope === clearScopes.selectedHistory) {
    const ids = new Set(input.ids || []);
    return records.map((record) => (ids.has(record.id) ? { ...record, hidden: true, clearedAt: now } : record));
  }

  return records;
}

export function visibleHistory(records) {
  return records.filter((record) => !record.hidden && !record.clearedAt);
}

