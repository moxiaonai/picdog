import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { writeClipboardText } from '../../app/clipboard';
import { localizedErrorMessage, localizedUiMessage } from '../../app/errorMessages';
import { picdogCommands } from '../../contracts/commands';
import { defaultCompressionOptions } from '../../contracts/compressionPresets';
import type { AppSettings, AssetState, Batch, BatchState, CompressionOptions, FileInput, Route } from '../../contracts/models';
import { TASK_EVENT_NAME, type TaskEvent } from '../../contracts/events';
import { canCopy, canReveal, canUpload, getBatchState } from './batchState';
import { demoFiles } from './mockData';

function compressionOptionsFromSettings(settings?: AppSettings): CompressionOptions {
  return {
    mode: settings?.compressionMode ?? defaultCompressionOptions.mode,
    quality: settings?.compressionQuality ?? defaultCompressionOptions.quality,
    encoderDirectory: settings?.encoderDirectory,
    maxWidth: settings?.maxWidth,
    maxHeight: settings?.maxHeight,
    outputFormat: settings?.outputFormat ?? defaultCompressionOptions.outputFormat,
    stripMetadata: settings?.stripMetadata ?? defaultCompressionOptions.stripMetadata,
    preserveColorProfile: settings?.preserveColorProfile ?? defaultCompressionOptions.preserveColorProfile,
    keepOriginal: settings?.keepOriginal ?? defaultCompressionOptions.keepOriginal,
    fallbackWhenLarger: settings?.fallbackWhenLarger ?? defaultCompressionOptions.fallbackWhenLarger
  };
}

type BatchControllerOptions = {
  settings?: AppSettings;
  routes?: Route[];
  onHistoryChanged?: () => void | Promise<void>;
};

function nextCompressedAsset(asset: AssetState): AssetState {
  const savedRatio = asset.mime?.includes('svg') ? 82 : asset.mime?.includes('gif') ? 61 : 74;
  const outputSize = Math.round(asset.sourceSize * (1 - savedRatio / 100));

  return {
    ...asset,
    outputPath: asset.sourcePath + '.picdog',
    outputSize,
    savedRatio,
    compressStatus: 'done',
    compressProgress: 100,
    selected: false
  };
}

function publicUrlForAsset(asset: AssetState, route?: Route) {
  const base = (route?.publicBaseUrl ?? 'https://cdn.picdog.app').replace(/\/$/, '');
  return base + '/2026/10/' + asset.name.replace(/\.[^.]+$/, '.webp');
}

function fallbackCompressedBatch(batch: Batch): Batch {
  const assets = batch.assets.map(nextCompressedAsset);

  return {
    ...batch,
    assets,
    status: getBatchState(assets),
    updatedAt: new Date().toISOString()
  };
}

function fallbackUploadedBatch(batch: Batch, assetIds: string[], routeId: string, route?: Route): Batch {
  const assets = batch.assets.map((asset) => {
    if (!assetIds.includes(asset.id) || !canUpload(asset)) return asset;

    return {
      ...asset,
      uploadStatus: 'done' as const,
      uploadProgress: 100,
      selected: false,
      routeId,
      publicUrl: publicUrlForAsset(asset, route)
    };
  });

  return {
    ...batch,
    assets,
    status: getBatchState(assets),
    updatedAt: new Date().toISOString()
  };
}

function hasAssets(batch: Batch) {
  return batch.assets.length > 0;
}

function shouldCompress(asset: AssetState) {
  return asset.compressStatus !== 'done' || !asset.outputPath;
}

function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function updateAsset(batch: Batch, assetId: string, mapper: (asset: AssetState) => AssetState): Batch {
  const assets = batch.assets.map((asset) => asset.id === assetId ? mapper(asset) : asset);
  return {
    ...batch,
    assets,
    status: getBatchState(assets),
    updatedAt: new Date().toISOString()
  };
}

function applyTaskEvent(current: Batch | null, event: TaskEvent): Batch | null {
  if (event.type === 'batch.created') return event.batch;
  if (!current || current.id !== event.batchId) return current;

  if (event.type === 'batch.status.changed') {
    return { ...current, status: event.status, updatedAt: new Date().toISOString() };
  }

  if (event.type === 'asset.compress.started') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      compressStatus: 'running',
      compressProgress: Math.max(asset.compressProgress, 1)
    }));
  }

  if (event.type === 'asset.compress.progress') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      compressStatus: 'running',
      compressProgress: event.progress
    }));
  }

  if (event.type === 'asset.compress.done') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      outputPath: event.output.outputPath ?? asset.outputPath,
      outputSize: event.output.outputSize ?? asset.outputSize,
      mime: event.output.mime ?? asset.mime,
      savedRatio: event.output.savedRatio ?? asset.savedRatio,
      width: event.output.width ?? asset.width,
      height: event.output.height ?? asset.height,
      compressStatus: 'done',
      compressProgress: 100,
      selected: false
    }));
  }

  if (event.type === 'asset.upload.started') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      uploadStatus: 'running',
      uploadProgress: Math.max(asset.uploadProgress, 1)
    }));
  }

  if (event.type === 'asset.upload.progress') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      uploadStatus: 'running',
      uploadProgress: event.progress
    }));
  }

  if (event.type === 'asset.upload.done') {
    return updateAsset(current, event.assetId, (asset) => ({
      ...asset,
      publicUrl: event.url,
      uploadStatus: 'done',
      uploadProgress: 100,
      selected: false
    }));
  }

  if (event.type === 'asset.cancelled') {
    return {
      ...updateAsset(current, event.assetId, (asset) => ({
        ...asset,
        compressStatus: event.stage === 'compress' ? 'cancelled' : asset.compressStatus,
        uploadStatus: event.stage === 'upload' ? 'cancelled' : asset.uploadStatus,
        compressProgress: event.stage === 'compress' ? 0 : asset.compressProgress,
        uploadProgress: event.stage === 'upload' ? 0 : asset.uploadProgress,
        selected: event.stage === 'upload' ? true : asset.selected
      })),
      status: 'cancelling'
    };
  }

  if (event.type === 'asset.failed') {
    return {
      ...updateAsset(current, event.assetId, (asset) => ({
        ...asset,
        compressStatus: event.stage === 'compress' ? 'failed' : asset.compressStatus,
        uploadStatus: event.stage === 'upload' ? 'failed' : asset.uploadStatus,
        error: event.error
      })),
      status: 'failed'
    };
  }

  return current;
}

export function useBatchController(options: BatchControllerOptions = {}) {
  const locale = options.settings?.locale ?? 'zh-CN';
  const uploadEnabled = options.settings?.uploadEnabled ?? false;
  const [batch, setBatch] = useState<Batch | null>(null);
  const [state, setState] = useState<BatchState>('empty');
  const [lastError, setLastError] = useState<string | null>(null);

  useEffect(() => {
    if (uploadEnabled) return;

    setBatch((current) => {
      if (!current || current.assets.every((asset) => !asset.selected)) return current;

      return {
        ...current,
        assets: current.assets.map((asset) =>
          asset.selected ? { ...asset, selected: false } : asset
        )
      };
    });
  }, [uploadEnabled]);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    let disposed = false;

    void picdogCommands
      .getLatestBatch()
      .then((latest) => {
        if (disposed || !latest || latest.assets.length === 0) return;

        setBatch((current) => {
          if (current) return current;
          setState(latest.status);
          return latest;
        });
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    const unlistenPromise = listen<TaskEvent>(TASK_EVENT_NAME, (event) => {
      if (event.payload.type === 'asset.failed') {
        setLastError(localizedErrorMessage(event.payload.error, locale));
      }
      if (event.payload.type === 'asset.cancelled') {
        setLastError(localizedUiMessage('operationCancelled', locale));
      }

      setBatch((current) => {
        const next = applyTaskEvent(current, event.payload);
        if (next) setState(next.status);
        return next;
      });
    });

    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [locale]);

  const assets = batch?.assets ?? [];
  const selectedCount = uploadEnabled ? assets.filter((asset) => asset.selected).length : 0;
  const uploadedCount = assets.filter(canCopy).length;
  const selectedUploadableCount = uploadEnabled
    ? assets.filter((asset) => asset.selected && canUpload(asset)).length
    : 0;
  const hasRetryableCompression = assets.some((asset) =>
    asset.compressStatus === 'failed' || asset.compressStatus === 'cancelled'
  );

  const importFiles = async (files: FileInput[]) => {
    if (files.length === 0) return;

    const nextBatch = await picdogCommands.createBatch(files);
    setLastError(null);
    setBatch(nextBatch);
    setState('queue');
  };

  const chooseDemoFiles = async () => {
    await importFiles(demoFiles);
  };

  const clearBatch = async () => {
    if (batch) {
      await picdogCommands.clearBatch(batch.id, options.settings?.deleteLocalOutputsOnClear ?? false);
    }

    setLastError(null);
    setBatch(null);
    setState('empty');
  };

  const cancelTask = async () => {
    if (!batch) return;

    try {
      setState('cancelling');
      setBatch((current) => current ? { ...current, status: 'cancelling' } : current);
      const syncedBatch = await picdogCommands.cancelTask(batch.id);
      if (hasAssets(syncedBatch)) {
        setBatch(syncedBatch);
        setState(syncedBatch.status);
      }
      setLastError(localizedUiMessage('operationCancelled', locale));
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
    }
  };

  const toggleAsset = (assetId: string) => {
    if (!uploadEnabled) return;

    setBatch((current) => current
      ? {
          ...current,
          assets: current.assets.map((asset) =>
            asset.id === assetId && canUpload(asset)
              ? { ...asset, selected: !asset.selected }
              : asset
          )
        }
      : current
    );
  };

  const selectAll = () => {
    if (!uploadEnabled) return;

    setBatch((current) => {
      if (!current) return current;

      const uploadable = current.assets.filter(canUpload);
      const shouldSelect = uploadable.some((asset) => !asset.selected);

      return {
        ...current,
        assets: current.assets.map((asset) =>
          canUpload(asset) ? { ...asset, selected: shouldSelect } : asset
        )
      };
    });
  };

  const startCompression = async () => {
    if (!batch) return;

    const currentBatch = batch;
    setState('compressing');
    setBatch({
      ...currentBatch,
      status: 'compressing',
      assets: currentBatch.assets.map((asset) => ({
        ...asset,
        compressStatus: shouldCompress(asset) ? 'running' : asset.compressStatus,
        compressProgress: shouldCompress(asset) ? Math.max(asset.compressProgress, 8) : asset.compressProgress
      }))
    });

    try {
      const syncedBatch = await picdogCommands.startCompression(currentBatch.id, compressionOptionsFromSettings(options.settings));
      const nextBatch = hasAssets(syncedBatch) ? syncedBatch : fallbackCompressedBatch(currentBatch);
      const hasCancelled = nextBatch.assets.some((asset) => asset.compressStatus === 'cancelled');
      setLastError(hasCancelled ? localizedUiMessage('operationCancelled', locale) : null);
      setBatch(nextBatch);
      setState(nextBatch.status);
      await options.onHistoryChanged?.();
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
      setState('failed');
      setBatch((current) => current ? { ...current, status: 'failed' } : current);
    }
  };

  const uploadAssetIds = async (assetIds: string[]) => {
    if (!uploadEnabled || !batch || assetIds.length === 0) return;

    const targets = batch.assets.filter((asset) => assetIds.includes(asset.id) && canUpload(asset));
    if (targets.length === 0) return;

    const currentBatch = batch;
    const uploadableIds = targets.map((asset) => asset.id);
    const route = options.routes?.find((item) => item.id === options.settings?.defaultRouteId) ?? options.routes?.[0];

    if (!route) {
      setLastError(localizedUiMessage('noUploadTarget', locale));
      return;
    }

    const routeId = route.id;

    setState('uploading');
    setBatch({
      ...currentBatch,
      status: 'uploading',
      assets: currentBatch.assets.map((asset) =>
        uploadableIds.includes(asset.id)
          ? {
              ...asset,
              uploadStatus: 'running',
              uploadProgress: Math.max(asset.uploadProgress, 10)
            }
          : asset
      )
    });

    try {
      const syncedBatch = await picdogCommands.uploadAssets(currentBatch.id, uploadableIds, routeId);
      const nextBatch = hasAssets(syncedBatch)
        ? syncedBatch
        : fallbackUploadedBatch(currentBatch, uploadableIds, routeId, route);
      const hasFailedUpload = nextBatch.assets.some((asset) =>
        uploadableIds.includes(asset.id) && asset.uploadStatus === 'failed'
      );
      const hasCancelledUpload = nextBatch.assets.some((asset) =>
        uploadableIds.includes(asset.id) && asset.uploadStatus === 'cancelled'
      );

      setLastError((current) => hasFailedUpload
        ? current ?? localizedUiMessage('partialUploadFailed', locale)
        : hasCancelledUpload
          ? localizedUiMessage('operationCancelled', locale)
        : null
      );
      setBatch(nextBatch);
      setState(nextBatch.status);
      await options.onHistoryChanged?.();
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
      setState('failed');
      setBatch((current) => current ? { ...current, status: 'failed' } : current);
    }
  };

  const uploadSelected = async () => {
    if (!uploadEnabled) return;
    await uploadAssetIds(batch?.assets.filter((asset) => asset.selected).map((asset) => asset.id) ?? []);
  };

  const uploadOne = async (assetId: string) => {
    if (!uploadEnabled) return;
    await uploadAssetIds([assetId]);
  };

  const copyAsset = async (assetId: string) => {
    if (!uploadEnabled) return;

    const asset = batch?.assets.find((item) => item.id === assetId);
    if (!asset?.publicUrl) return;

    try {
      const text = await picdogCommands.copyLink(assetId, options.settings?.copyFormat ?? 'markdown', {
        publicUrl: asset.publicUrl,
        fileName: asset.name,
        alt: asset.name,
        width: asset.width,
        height: asset.height,
        customTemplate: options.routes?.find((route) => route.id === asset.routeId)?.customLinkTemplate
      });

      await writeClipboardText(text || asset.publicUrl);
      await picdogCommands.markLinkCopied(assetId);
      setLastError(null);
      void options.onHistoryChanged?.();
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
    }
  };

  const copyAllLinks = async () => {
    if (!uploadEnabled) return;

    const uploadedAssets = batch?.assets.filter(canCopy) ?? [];
    if (uploadedAssets.length === 0) return;

    try {
      const links: string[] = [];
      for (const asset of uploadedAssets) {
        const text = await picdogCommands.copyLink(asset.id, options.settings?.copyFormat ?? 'markdown', {
          publicUrl: asset.publicUrl,
          fileName: asset.name,
          alt: asset.name,
          width: asset.width,
          height: asset.height,
          customTemplate: options.routes?.find((route) => route.id === asset.routeId)?.customLinkTemplate
        });
        links.push(text || asset.publicUrl || '');
      }

      await writeClipboardText(links.filter(Boolean).join('\n'));
      await picdogCommands.markLinksCopied(uploadedAssets.map((asset) => asset.id));
      setLastError(null);
      void options.onHistoryChanged?.();
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
    }
  };

  const revealAsset = async (assetId: string) => {
    const asset = batch?.assets.find((item) => item.id === assetId);
    if (!asset || !canReveal(asset)) return;

    try {
      await picdogCommands.revealOutput(assetId);
      setLastError(null);
    } catch (error) {
      setLastError(localizedErrorMessage(error, locale));
    }
  };

  const primaryAction = async () => {
    if (state === 'empty') return chooseDemoFiles();
    if (state === 'queue') return startCompression();
    if (uploadEnabled && state === 'compressed') return uploadSelected();
    if (uploadEnabled && state === 'failed' && selectedUploadableCount > 0) return uploadSelected();
    if (state === 'failed' && hasRetryableCompression) return startCompression();
    if (state === 'uploaded') return chooseDemoFiles();
  };

  const canPrimary =
    state === 'empty' ||
    state === 'queue' ||
    state === 'uploaded' ||
    (uploadEnabled && state === 'compressed' && selectedUploadableCount > 0 && Boolean(options.routes?.length)) ||
    (uploadEnabled && state === 'failed' && selectedUploadableCount > 0 && Boolean(options.routes?.length)) ||
    (state === 'failed' && hasRetryableCompression);

  return {
    batch,
    assets,
    state,
    lastError,
    selectedCount,
    uploadedCount,
    canPrimary,
    canCopyAll: uploadEnabled && uploadedCount > 0,
    canSelectAll: uploadEnabled && assets.some(canUpload),
    canClear: ['queue', 'compressed', 'uploaded', 'failed'].includes(state),
    canCancel: state === 'compressing' || state === 'uploading',
    primaryAction,
    chooseDemoFiles,
    importFiles,
    startCompression,
    uploadSelected,
    uploadOne,
    copyAsset,
    copyAllLinks,
    revealAsset,
    cancelTask,
    clearBatch,
    toggleAsset,
    selectAll,
    primaryActionKind: useMemo<'choose' | 'compress' | 'retryCompression' | 'uploadSelected' | 'busy'>(() => {
      if (state === 'empty') return 'choose';
      if (state === 'queue') return 'compress';
      if (uploadEnabled && state === 'compressed') return 'uploadSelected';
      if (uploadEnabled && state === 'failed' && selectedUploadableCount > 0) return 'uploadSelected';
      if (state === 'failed' && hasRetryableCompression) return 'retryCompression';
      if (state === 'uploaded') return 'choose';
      return 'busy';
    }, [hasRetryableCompression, selectedUploadableCount, state, uploadEnabled])
  };
}



