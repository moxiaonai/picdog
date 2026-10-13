import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettings,
  Batch,
  CompareImagePayload,
  ClearHistoryInput,
  CompressionOptions,
  CompressionToolStatus,
  CopyFallback,
  CopyFormat,
  FileInput,
  HistoryItem,
  HistoryPage,
  HistoryQuery,
  Route,
  RouteCredentialInput,
  RouteCredentialStatus,
  RouteDraft,
  RouteTestResult
} from './models';
import { createMockBatch, mockHistory, mockRoutes, mockSettings } from '../features/batch/mockData';

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let previewSettings: AppSettings = { ...mockSettings };
let previewRoutes: Route[] = [...mockRoutes];
let previewHistory: HistoryItem[] = [...mockHistory];

async function callCommand<T>(name: string, args: Record<string, unknown>, fallback: () => T | Promise<T>): Promise<T> {
  if (hasTauriRuntime()) {
    return invoke<T>(name, args);
  }

  return fallback();
}

function formatPublicLink(format: CopyFormat, fallback?: CopyFallback) {
  const publicUrl = fallback?.publicUrl ?? '';
  const fileName = fallback?.fileName ?? 'image';
  const alt = fallback?.alt ?? fileName;
  const width = fallback?.width ? String(fallback.width) : '';
  const height = fallback?.height ? String(fallback.height) : '';

  if (format === 'markdown') return `![${alt}](${publicUrl})`;
  if (format === 'html') {
    const size = width && height ? ` width="${width}" height="${height}"` : '';
    return `<img src="${publicUrl}" alt="${alt}"${size} />`;
  }
  if (format === 'custom' && fallback?.customTemplate) {
    return fallback.customTemplate
      .replaceAll('{url}', publicUrl)
      .replaceAll('{name}', fileName)
      .replaceAll('{alt}', alt)
      .replaceAll('{width}', width)
      .replaceAll('{height}', height);
  }

  return publicUrl;
}

function fileInputFromPath(path: string): FileInput {
  const name = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
  const extension = name.split('.').pop()?.toLowerCase();
  const mimeByExtension: Record<string, string> = {
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    webp: 'image/webp',
    svg: 'image/svg+xml',
    avif: 'image/avif'
  };

  return {
    path,
    name,
    size: 0,
    mime: extension ? mimeByExtension[extension] : undefined,
    isDirectory: false
  };
}

export const picdogCommands = {
  checkCompressionTools(encoderDirectory?: string): Promise<CompressionToolStatus[]> {
    return callCommand('check_compression_tools', { encoderDirectory }, () => [
      { name: 'cjpeg', available: false, purpose: 'JPEG / MozJPEG compatible encoding' },
      { name: 'cwebp', available: false, purpose: 'WebP encoding' },
      { name: 'gif2webp', available: false, purpose: 'Animated GIF to WebP encoding' }
    ]);
  },

  selectFiles(title?: string): Promise<FileInput[]> {
    return callCommand('select_files', { title }, () => []);
  },

  scanDroppedPaths(paths: string[]): Promise<FileInput[]> {
    return callCommand('scan_dropped_paths', { paths }, () => paths.map(fileInputFromPath));
  },

  createBatch(files: FileInput[]): Promise<Batch> {
    return callCommand('create_batch', { files }, () => createMockBatch(files));
  },

  getBatch(batchId: string): Promise<Batch | null> {
    return callCommand('get_batch', { batchId }, () => null);
  },

  getLatestBatch(): Promise<Batch | null> {
    return callCommand('get_latest_batch', {}, () => null);
  },

  readImagePreview(path: string): Promise<string | null> {
    return callCommand('read_image_preview', { path }, () => null);
  },

  readCompareImages(assetId: string): Promise<CompareImagePayload> {
    return callCommand('read_compare_images', { assetId }, () => ({
      assetId,
      fileName: 'preview.png',
      sourceSize: 0,
      beforeDataUrl: undefined,
      afterDataUrl: undefined
    }));
  },

  startCompression(batchId: string, options: CompressionOptions): Promise<Batch> {
    return callCommand('start_compression', { batchId, options }, () => createMockBatch([]));
  },

  uploadAssets(batchId: string, assetIds: string[], routeId: string): Promise<Batch> {
    return callCommand('upload_assets', { batchId, assetIds, routeId }, () => createMockBatch([]));
  },

  copyLink(assetId: string, format: CopyFormat, fallback?: CopyFallback): Promise<string> {
    return callCommand('copy_link', { assetId, format }, () => formatPublicLink(format, fallback));
  },

  markLinkCopied(assetId: string): Promise<void> {
    return callCommand('mark_link_copied', { assetId }, () => undefined);
  },

  markLinksCopied(assetIds: string[]): Promise<void> {
    return callCommand('mark_links_copied', { assetIds }, () => undefined);
  },

  clearBatch(batchId: string, deleteLocalOutputs = false): Promise<void> {
    return callCommand('clear_batch', { batchId, deleteLocalOutputs }, () => undefined);
  },

  cancelTask(batchId: string): Promise<Batch> {
    return callCommand('cancel_task', { batchId }, () => createMockBatch([]));
  },

  revealOutput(assetId: string): Promise<void> {
    return callCommand('reveal_output', { assetId }, () => undefined);
  },

  listRoutes(): Promise<Route[]> {
    return callCommand('list_routes', {}, () => [...previewRoutes]);
  },

  selectDirectory(title?: string): Promise<string | null> {
    return callCommand('select_directory', { title }, () => null);
  },

  saveRoute(draft: RouteDraft): Promise<Route> {
    return callCommand('save_route', { draft }, () => {
      const routeId = draft.id || draft.name.toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-|-$/g, '') || 'route-preview';
      const existingRoute = previewRoutes.find((route) => route.id === routeId);
      const now = new Date().toISOString();

      const route = {
        ...draft,
        id: routeId,
        credentialRef: existingRoute?.credentialRef ?? 'keychain://picdog/' + routeId,
        createdAt: existingRoute?.createdAt ?? now,
        updatedAt: now
      };

      previewRoutes = [
        ...previewRoutes.filter((item) => item.id !== routeId),
        route
      ];
      previewSettings = { ...previewSettings, defaultRouteId: route.id };

      return route;
    });
  },

  deleteRoute(routeId: string): Promise<Route[]> {
    return callCommand('delete_route', { routeId }, () => {
      previewRoutes = previewRoutes.filter((route) => route.id !== routeId);
      if (previewSettings.defaultRouteId === routeId) {
        previewSettings = { ...previewSettings, defaultRouteId: previewRoutes[0]?.id };
      }

      return [...previewRoutes];
    });
  },

  saveRouteCredential(routeId: string, credential: RouteCredentialInput): Promise<RouteCredentialStatus> {
    return callCommand('save_route_credential', { routeId, credential }, () => ({
      routeId,
      credentialRef: 'keychain://picdog/preview',
      configured: Boolean(credential.accessKeyId || credential.secretAccessKey || credential.apiToken),
      checkedAt: new Date().toISOString()
    }));
  },

  getRouteCredentialStatus(routeId: string): Promise<RouteCredentialStatus> {
    return callCommand('get_route_credential_status', { routeId }, () => ({
      routeId,
      credentialRef: 'keychain://picdog/preview',
      configured: false,
      checkedAt: new Date().toISOString()
    }));
  },

  deleteRouteCredential(routeId: string): Promise<RouteCredentialStatus> {
    return callCommand('delete_route_credential', { routeId }, () => ({
      routeId,
      credentialRef: 'keychain://picdog/preview',
      configured: false,
      checkedAt: new Date().toISOString()
    }));
  },

  testRoute(routeId: string): Promise<RouteTestResult> {
    const route = previewRoutes.find((item) => item.id === routeId);

    return callCommand('test_route', { routeId }, () => ({
      routeId,
      ok: Boolean(route),
      messageCode: route ? 'route_test_success' : 'route_not_found',
      message: route ? 'Route is reachable' : 'No upload target',
      checkedAt: new Date().toISOString(),
      publicBaseUrl: route?.publicBaseUrl
    }));
  },

  getSettings(): Promise<AppSettings> {
    return callCommand('get_settings', {}, () => previewSettings);
  },

  saveSettings(settings: AppSettings): Promise<void> {
    return callCommand('save_settings', { settings }, () => {
      previewSettings = { ...settings };
    });
  },

  listHistory(query: HistoryQuery): Promise<HistoryPage> {
    return callCommand('list_history', { query }, () => {
      let items = [...previewHistory];
      const keyword = query.keyword?.trim().toLowerCase();

      if (keyword) {
        items = items.filter((item) => item.fileName.toLowerCase().includes(keyword));
      }

      if (query.routeId) {
        items = items.filter((item) => item.routeId === query.routeId);
      }

      if (query.status === 'uploaded') {
        items = items.filter((item) => item.canCopy);
      } else if (query.status === 'compressed') {
        items = items.filter((item) => !item.canCopy && item.compressStatus === 'done');
      } else if (query.status === 'failed') {
        items = items.filter((item) =>
          item.compressStatus === 'failed' ||
          item.compressStatus === 'cancelled' ||
          item.uploadStatus === 'failed' ||
          item.uploadStatus === 'cancelled'
        );
      }

      const page = Math.max(1, query.page);
      const pageSize = Math.min(Math.max(1, query.pageSize), 100);
      const start = (page - 1) * pageSize;

      return {
        items: items.slice(start, start + pageSize),
        page,
        pageSize,
        total: items.length
      };
    });
  },

  clearHistory(input: ClearHistoryInput): Promise<void> {
    return callCommand('clear_history', { input }, () => {
      if (input.scope === 'selected' && input.ids?.length) {
        previewHistory = previewHistory.filter((item) => !input.ids?.includes(item.id));
        return;
      }

      previewHistory = [];
    });
  },

  exportHistory(query: HistoryQuery, title?: string): Promise<string> {
    return callCommand('export_history', { query, title }, () => 'picdog-history.csv');
  }
};


