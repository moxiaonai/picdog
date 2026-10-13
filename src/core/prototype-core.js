(function initPicDogCore(root, factory) {
  const api = factory();

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  }

  root.PicDogCore = api;
})(typeof globalThis !== 'undefined' ? globalThis : window, function createPicDogCore() {
  function cloneFiles(initialFiles) {
    return initialFiles.map(file => ({
      ...file,
      selected: false,
      compress: 0,
      upload: 0
    }));
  }

  function normalizeUploadEnabled(value) {
    return value === true || value === 'true';
  }

  function selectedCount(files) {
    return files.filter(file => file.selected).length;
  }

  function uploadedCount(files) {
    return files.filter(file => file.upload === 100).length;
  }

  function hasPendingUploads(files) {
    return uploadedCount(files) < files.length;
  }

  function isRowLocked(state) {
    return state === 'queue' || state === 'compressing' || state === 'uploading';
  }

  function canUpload(state, file) {
    return (state === 'compressed' || state === 'uploaded') && file.upload < 100;
  }

  function canCopy(file) {
    return file.upload === 100;
  }

  function setAllCompression(files, progress) {
    files.forEach(file => {
      file.compress = progress;
    });
    return files;
  }

  function setUploadProgress(files, indexes, progress) {
    indexes.forEach(index => {
      if (files[index]) files[index].upload = progress;
    });
    return files;
  }

  function clearSelection(files) {
    files.forEach(file => {
      file.selected = false;
    });
    return files;
  }

  function toggleSelectableFiles(files) {
    const selectable = files.filter(file => file.upload < 100);
    const allSelected = selectable.length > 0 && selectable.every(file => file.selected);

    files.forEach(file => {
      file.selected = file.upload < 100 ? !allSelected : false;
    });

    return files;
  }

  function selectedUploadIndexes(files) {
    return files
      .map((file, index) => file.selected && file.upload < 100 ? index : null)
      .filter(index => index !== null);
  }

  function nextStateAfterUpload(files) {
    return uploadedCount(files) === files.length ? 'uploaded' : 'compressed';
  }

  function makePublicUrl(fileName, targetName) {
    const base = targetName.includes('Docs')
      ? 'https://docs-cdn.picdog.app'
      : 'https://cdn.picdog.app';
    const outputName = fileName.replace(/\.[^.]+$/, '.webp');
    return base + '/2026/10/' + encodeURIComponent(outputName);
  }

  function historyRecordFromFile(file, index, targetName) {
    const uploaded = file.upload === 100;

    return {
      id: 'current-' + index + '-' + file.name,
      name: file.name,
      meta: targetName + ' · ' + file.saved,
      statusKey: uploaded ? 'historyUploaded' : 'historyCompressed',
      url: uploaded ? makePublicUrl(file.name, targetName) : ''
    };
  }

  function upsertHistoryFromFiles(historyRecords, files, targetName, limit) {
    let nextRecords = [...historyRecords];

    files.forEach((file, index) => {
      if (file.compress < 100) return;

      const record = historyRecordFromFile(file, index, targetName);
      const existingIndex = nextRecords.findIndex(item => item.id === record.id);

      if (existingIndex >= 0) {
        nextRecords[existingIndex] = record;
      } else {
        nextRecords.unshift(record);
      }
    });

    return nextRecords.slice(0, limit || 8);
  }

  return {
    cloneFiles,
    normalizeUploadEnabled,
    selectedCount,
    uploadedCount,
    hasPendingUploads,
    isRowLocked,
    canUpload,
    canCopy,
    setAllCompression,
    setUploadProgress,
    clearSelection,
    toggleSelectableFiles,
    selectedUploadIndexes,
    nextStateAfterUpload,
    makePublicUrl,
    historyRecordFromFile,
    upsertHistoryFromFiles
  };
});
