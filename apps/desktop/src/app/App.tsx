import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent, type DragEvent } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { WindowFrame } from '../components/WindowFrame';
import { writeClipboardText } from './clipboard';
import { localizedErrorMessage } from './errorMessages';
import { createTranslator } from './i18n';
import type {
  AppSettings,
  CompressionToolStatus,
  CopyFormat,
  FileInput,
  HistoryItem,
  HistoryQuery,
  Locale,
  Route,
  RouteCredentialInput,
  RouteDraft,
  RouteTestResult
} from '../contracts/models';
import { mockRoutes, mockSettings } from '../features/batch/mockData';
import { useBatchController } from '../features/batch/useBatchController';
import { FileList as BatchFileList } from '../features/batch/FileList';
import { ActionBar } from '../features/batch/ActionBar';
import { SettingsPopover } from '../features/settings/SettingsPopover';
import { RouteSettingsPopover } from '../features/settings/RouteSettingsPopover';
import { HistoryPopover, type HistoryFilters } from '../features/history/HistoryPopover';
import { picdogCommands } from '../contracts/commands';

const supportedExtensions = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'avif']);
const historyPageSize = 20;

function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function fileInputFromBrowserFile(file: File): FileInput | null {
  const extension = file.name.split('.').pop()?.toLowerCase();
  if (!extension || !supportedExtensions.has(extension)) return null;

  return {
    path: file.name,
    name: file.name,
    size: file.size,
    mime: file.type || undefined,
    previewUrl: URL.createObjectURL(file),
    isDirectory: false
  };
}

function fileInputsFromBrowserFiles(files: globalThis.FileList | File[]): FileInput[] {
  const seen = new Set<string>();
  return Array.from(files)
    .map(fileInputFromBrowserFile)
    .filter((file): file is FileInput => Boolean(file))
    .filter((file) => {
      const key = [file.path, file.name, file.size].join('|');
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

export function App() {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [settings, setSettings] = useState<AppSettings>(mockSettings);
  const [routes, setRoutes] = useState<Route[]>(mockRoutes);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [routeSettingsOpen, setRouteSettingsOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [compressionTools, setCompressionTools] = useState<CompressionToolStatus[]>([]);
  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]);
  const [historyTotal, setHistoryTotal] = useState(0);
  const [historyPage, setHistoryPage] = useState(1);
  const [historyFilters, setHistoryFilters] = useState<HistoryFilters>({
    keyword: '',
    routeId: '',
    status: ''
  });
  const historyFiltersRef = useRef(historyFilters);
  const [historySelection, setHistorySelection] = useState<string[]>([]);
  const [historyClearLocalOutputs, setHistoryClearLocalOutputs] = useState(false);
  const [historyStatus, setHistoryStatus] = useState<string | null>(null);

  const loadHistory = useCallback(async (
    filters = historyFiltersRef.current,
    pageNumber = 1,
    append = false
  ) => {
    const query: HistoryQuery = {
      keyword: filters.keyword.trim() || undefined,
      routeId: filters.routeId || undefined,
      status: filters.status || undefined,
      page: pageNumber,
      pageSize: historyPageSize
    };
    const page = await picdogCommands.listHistory(query);

    setHistoryItems((current) => append ? [...current, ...page.items] : page.items);
    setHistoryTotal(page.total);
    setHistoryPage(page.page);
    if (!append) {
      setHistorySelection([]);
    }
  }, []);

  const loadSettings = useCallback(async () => {
    const nextSettings = await picdogCommands.getSettings();
    setSettings(nextSettings);
    document.documentElement.lang = nextSettings.locale;
  }, []);

  const loadRoutes = useCallback(async () => {
    const nextRoutes = await picdogCommands.listRoutes();
    setRoutes(nextRoutes);
  }, []);

  const loadCompressionTools = useCallback(async (encoderDirectory = settings.encoderDirectory) => {
    const tools = await picdogCommands.checkCompressionTools(encoderDirectory);
    setCompressionTools(tools);
  }, [settings.encoderDirectory]);

  useEffect(() => {
    void loadSettings();
    void loadRoutes();
    void loadCompressionTools();
    void loadHistory();
  }, [loadCompressionTools, loadHistory, loadRoutes, loadSettings]);

  const updateSettings = useCallback((patch: Partial<AppSettings>) => {
    setSettings((current) => {
      const next = { ...current, ...patch };
      if (patch.locale) {
        document.documentElement.lang = patch.locale;
      }
      void picdogCommands.saveSettings(next);
      return next;
    });
  }, []);

  const saveRoute = useCallback(async (draft: RouteDraft) => {
    const route = await picdogCommands.saveRoute(draft);
    await loadRoutes();
    updateSettings({ defaultRouteId: route.id });
  }, [loadRoutes, updateSettings]);

  const deleteRoute = useCallback(async (routeId: string) => {
    const nextRoutes = await picdogCommands.deleteRoute(routeId);
    const nextSettings = await picdogCommands.getSettings();
    const defaultRouteId = nextRoutes.some((route) => route.id === nextSettings.defaultRouteId)
      ? nextSettings.defaultRouteId
      : nextRoutes[0]?.id;

    setRoutes(nextRoutes);
    setSettings({ ...nextSettings, defaultRouteId });
    document.documentElement.lang = nextSettings.locale;

    if (defaultRouteId !== nextSettings.defaultRouteId) {
      void picdogCommands.saveSettings({ ...nextSettings, defaultRouteId });
    }
  }, []);

  const saveRouteCredential = useCallback(async (routeId: string, credential: RouteCredentialInput) => {
    await picdogCommands.saveRouteCredential(routeId, credential);
  }, []);

  const deleteRouteCredential = useCallback(async (routeId: string) => {
    return picdogCommands.deleteRouteCredential(routeId);
  }, []);

  const getRouteCredentialStatus = useCallback(async (routeId: string) => {
    return picdogCommands.getRouteCredentialStatus(routeId);
  }, []);

  const testRoute = useCallback(async (routeId: string): Promise<RouteTestResult> => {
    return picdogCommands.testRoute(routeId);
  }, []);

  const selectDirectory = useCallback(async (title?: string) => {
    return picdogCommands.selectDirectory(title);
  }, []);

  const t = useMemo(() => createTranslator(settings.locale), [settings.locale]);

  const updateHistoryFilters = useCallback((patch: Partial<HistoryFilters>) => {
    setHistoryFilters((current) => {
      const next = { ...current, ...patch };
      historyFiltersRef.current = next;
      setHistoryStatus(null);
      void loadHistory(next);
      return next;
    });
  }, [loadHistory]);

  const toggleHistorySelection = useCallback((itemId: string) => {
    setHistorySelection((current) =>
      current.includes(itemId)
        ? current.filter((id) => id !== itemId)
        : [...current, itemId]
    );
  }, []);

  const clearHistory = useCallback(async () => {
    const ids = historySelection;

    try {
      await picdogCommands.clearHistory({
        scope: ids.length > 0 ? 'selected' : 'all',
        ids: ids.length > 0 ? ids : undefined,
        clearLocalOutputs: historyClearLocalOutputs
      });
      await loadHistory();
      setHistoryStatus(t('historyCleared'));
    } catch (error) {
      setHistoryStatus(localizedErrorMessage(error, settings.locale));
    }
  }, [historyClearLocalOutputs, historySelection, loadHistory, settings.locale, t]);

  const loadMoreHistory = useCallback(async () => {
    await loadHistory(historyFiltersRef.current, historyPage + 1, true);
  }, [historyPage, loadHistory]);

  const exportHistory = useCallback(async () => {
    const filters = historyFiltersRef.current;

    try {
      await picdogCommands.exportHistory({
        keyword: filters.keyword.trim() || undefined,
        routeId: filters.routeId || undefined,
        status: filters.status || undefined,
        page: 1,
        pageSize: historyPageSize
      }, t('exportHistoryTitle'));
      setHistoryStatus(t('historyExported'));
    } catch (error) {
      setHistoryStatus(localizedErrorMessage(error, settings.locale));
    }
  }, [settings.locale, t]);

  const batch = useBatchController({ settings, routes, onHistoryChanged: loadHistory });
  const importFilesRef = useRef(batch.importFiles);
  const needsUploadTarget =
    settings.uploadEnabled &&
    routes.length === 0 &&
    batch.selectedCount > 0 &&
    (batch.state === 'compressed' || batch.state === 'failed');

  useEffect(() => {
    importFilesRef.current = batch.importFiles;
  }, [batch.importFiles]);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void getCurrentWebview()
      .onDragDropEvent(async (event) => {
        const payload = event.payload as { type: string; paths?: string[] };
        if (payload.type !== 'drop' || !payload.paths?.length) return;

        const files = await picdogCommands.scanDroppedPaths(payload.paths);
        if (!disposed) {
          await importFilesRef.current(files);
        }
      })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const copyHistoryItem = async (item: HistoryItem) => {
    if (!settings.uploadEnabled || !item.canCopy || !item.publicUrl) return;

    const text = await picdogCommands.copyLink(item.id, settings.copyFormat, {
      publicUrl: item.publicUrl,
      fileName: item.fileName,
      alt: item.fileName,
      width: item.width,
      height: item.height,
      customTemplate: routes.find((route) => route.id === item.routeId)?.customLinkTemplate
    });

    await writeClipboardText(text || item.publicUrl);
    await picdogCommands.markLinkCopied(item.id);
    await loadHistory();
  };

  const revealHistoryItem = async (item: HistoryItem) => {
    if (!item.outputPath) return;

    await picdogCommands.revealOutput(item.id);
  };

  const importBrowserFiles = async (files: globalThis.FileList | File[]) => {
    await batch.importFiles(fileInputsFromBrowserFiles(files));
  };

  const chooseFiles = async () => {
    if (hasTauriRuntime()) {
      const files = await picdogCommands.selectFiles(t('chooseImagesTitle'));
      await batch.importFiles(files);
      return;
    }

    fileInputRef.current?.click();
  };

  const handleFileInputChange = async (event: ChangeEvent<HTMLInputElement>) => {
    if (event.target.files) {
      await importBrowserFiles(event.target.files);
    }

    event.target.value = '';
  };

  const handleDrop = async (event: DragEvent<HTMLButtonElement>) => {
    event.preventDefault();
    await importBrowserFiles(event.dataTransfer.files);
  };

  const handleDragOver = (event: DragEvent<HTMLButtonElement>) => {
    event.preventDefault();
  };

  const handlePrimaryAction = () => {
    if (batch.state === 'empty' || batch.state === 'uploaded') {
      void chooseFiles();
      return;
    }

    void batch.primaryAction();
  };

  const closePanels = useCallback(() => {
    setSettingsOpen(false);
    setRouteSettingsOpen(false);
    setHistoryOpen(false);
  }, []);

  const openSettingsPanel = useCallback(() => {
    setSettingsOpen((current) => {
      const next = !current;
      if (next) {
        setRouteSettingsOpen(false);
        setHistoryOpen(false);
      }
      return next;
    });
  }, []);

  const openRouteSettingsPanel = useCallback(() => {
    setRouteSettingsOpen((current) => {
      const next = !current;
      if (next) {
        setSettingsOpen(false);
        setHistoryOpen(false);
      }
      return next;
    });
  }, []);

  const openHistoryPanel = useCallback(() => {
    setHistoryOpen((current) => {
      const next = !current;
      if (next) {
        setRouteSettingsOpen(false);
        setSettingsOpen(false);
        void loadHistory();
      }
      return next;
    });
  }, [loadHistory]);

  useEffect(() => {
    if (!settingsOpen && !routeSettingsOpen && !historyOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (target.closest('.popover') || target.closest('.title-actions')) return;

      closePanels();

      if (target.closest('.traffic')) return;

      const blockNextOutsideClick = (clickEvent: MouseEvent) => {
        const clickTarget = clickEvent.target;
        document.removeEventListener('click', blockNextOutsideClick, true);
        window.clearTimeout(blockCleanupTimeout);

        if (!(clickTarget instanceof Element)) return;
        if (clickTarget.closest('.popover') || clickTarget.closest('.title-actions') || clickTarget.closest('.traffic')) return;

        clickEvent.preventDefault();
        clickEvent.stopPropagation();
      };

      const blockCleanupTimeout = window.setTimeout(() => {
        document.removeEventListener('click', blockNextOutsideClick, true);
      }, 600);

      document.addEventListener('click', blockNextOutsideClick, true);
      event.preventDefault();
      event.stopPropagation();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closePanels();
      }
    };

    document.addEventListener('pointerdown', handlePointerDown, true);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [closePanels, historyOpen, routeSettingsOpen, settingsOpen]);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    let disposed = false;
    const unlisteners: Array<() => void> = [];

    void Promise.all([
      listen('picdog://open-settings', () => openSettingsPanel()),
      listen('picdog://open-cdn-settings', () => openRouteSettingsPanel()),
      listen('picdog://open-history', () => openHistoryPanel())
    ])
      .then((nextUnlisteners) => {
        if (disposed) {
          nextUnlisteners.forEach((unlisten) => unlisten());
          return;
        }

        unlisteners.push(...nextUnlisteners);
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [openHistoryPanel, openRouteSettingsPanel, openSettingsPanel]);

  return (
    <WindowFrame
      cdnLabel={t('cdnSettings')}
      settingsLabel={t('settings')}
      historyLabel={t('history')}
      onOpenCdnSettings={openRouteSettingsPanel}
      onOpenSettings={openSettingsPanel}
      onOpenHistory={openHistoryPanel}
    >
      <SettingsPopover
        open={settingsOpen}
        settings={settings}
        compressionTools={compressionTools}
        t={t}
        onLocaleChange={(locale: Locale) => updateSettings({ locale })}
        onCopyFormatChange={(copyFormat: CopyFormat) => updateSettings({ copyFormat })}
        onSettingsChange={updateSettings}
        onRefreshCompressionTools={loadCompressionTools}
        onSelectDirectory={selectDirectory}
        onClose={() => setSettingsOpen(false)}
      />
      <RouteSettingsPopover
        open={routeSettingsOpen}
        settings={settings}
        routes={routes}
        t={t}
        onUploadEnabledChange={(uploadEnabled: boolean) => updateSettings({ uploadEnabled })}
        onRouteChange={(defaultRouteId: string) => updateSettings({ defaultRouteId })}
        onSaveRoute={saveRoute}
        onDeleteRoute={deleteRoute}
        onSaveCredential={saveRouteCredential}
        onDeleteCredential={deleteRouteCredential}
        onGetCredentialStatus={getRouteCredentialStatus}
        onTestRoute={testRoute}
        onSelectDirectory={selectDirectory}
        onClose={() => setRouteSettingsOpen(false)}
      />
      <HistoryPopover
        open={historyOpen}
        items={historyItems}
        routes={routes}
        filters={historyFilters}
        selectedIds={historySelection}
        total={historyTotal}
        hasMore={historyItems.length < historyTotal}
        clearLocalOutputs={historyClearLocalOutputs}
        statusMessage={historyStatus}
        uploadEnabled={settings.uploadEnabled}
        t={t}
        onCopy={copyHistoryItem}
        onFiltersChange={updateHistoryFilters}
        onToggleSelect={toggleHistorySelection}
        onToggleClearLocalOutputs={setHistoryClearLocalOutputs}
        onClear={clearHistory}
        onReveal={revealHistoryItem}
        onExport={exportHistory}
        onLoadMore={loadMoreHistory}
        onClose={() => setHistoryOpen(false)}
      />

      <input
        ref={fileInputRef}
        className="visually-hidden"
        type="file"
        accept="image/jpeg,image/png,image/gif,image/webp,image/svg+xml,image/avif"
        multiple
        onChange={handleFileInputChange}
      />

      <section className="workspace" aria-live="polite">
        {batch.assets.length === 0 ? (
          <button
            className="drop-zone"
            type="button"
            onClick={() => void chooseFiles()}
            onDrop={handleDrop}
            onDragOver={handleDragOver}
          >
            <div className="drop-empty-card">
              <div className="drop-cloud" aria-hidden="true">
                <svg viewBox="0 0 64 48">
                  <path d="M22.2 39H49a11 11 0 0 0 1.8-21.8A17.2 17.2 0 0 0 18 15.4 12 12 0 0 0 22.2 39Z" />
                  <path d="M32 36V21" />
                  <path d="m24.5 28.4 7.5-7.5 7.5 7.5" />
                </svg>
              </div>
              <p className="drop-guide">
                <span>{t('dropGuidePrefix')}</span>
                <strong>{t('dropGuideAction')}</strong>
              </p>
              <p className="lead">{t(settings.uploadEnabled ? 'dropDesc' : 'dropDescCompressionOnly')}</p>
              <div className="format-row">{t('support')}</div>
            </div>
          </button>
        ) : (
          <BatchFileList
            assets={batch.assets}
            uploadEnabled={settings.uploadEnabled}
            t={t}
            locale={settings.locale}
            onToggle={batch.toggleAsset}
            onUploadOne={batch.uploadOne}
            onCopy={batch.copyAsset}
            onReveal={batch.revealAsset}
          />
        )}
      </section>

      <ActionBar
        state={batch.state}
        assets={batch.assets}
        uploadEnabled={settings.uploadEnabled}
        error={batch.lastError ?? (needsUploadTarget ? t('uploadTargetRequiredHint') : null)}
        canClear={batch.canClear}
        canCancel={batch.canCancel}
        canCopyAll={batch.canCopyAll}
        canSelectAll={batch.canSelectAll}
        canPrimary={batch.canPrimary}
        primaryActionKind={batch.primaryActionKind}
        t={t}
        onPrimary={handlePrimaryAction}
        onCancel={batch.cancelTask}
        onCopyAll={batch.copyAllLinks}
        onClear={batch.clearBatch}
        onSelectAll={batch.selectAll}
      />
    </WindowFrame>
  );
}







