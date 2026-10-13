import { useEffect, useMemo, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { createTranslator, type I18nKey } from '../../app/i18n';
import type { CompareImagePayload, Locale } from '../../contracts/models';
import { picdogCommands } from '../../contracts/commands';
import { formatBytes } from './batchState';

function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function localeFromUrl(): Locale {
  const locale = new URLSearchParams(window.location.search).get('locale');
  return locale === 'en-US' || locale === 'ja-JP' || locale === 'zh-CN' ? locale : 'zh-CN';
}

function assetIdFromUrl() {
  return new URLSearchParams(window.location.search).get('assetId') ?? '';
}

async function closeWindow() {
  if (!hasTauriRuntime()) {
    window.close();
    return;
  }

  await getCurrentWindow().close();
}

async function startWindowDrag() {
  if (!hasTauriRuntime()) return;
  await getCurrentWindow().startDragging();
}

export function CompareWindow() {
  const locale = useMemo(localeFromUrl, []);
  const t = useMemo(() => createTranslator(locale), [locale]);
  const assetId = useMemo(assetIdFromUrl, []);
  const [payload, setPayload] = useState<CompareImagePayload | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.classList.add('compare-window-root');
    return () => document.documentElement.classList.remove('compare-window-root');
  }, [locale]);

  useEffect(() => {
    let disposed = false;

    if (!assetId) {
      setError(t('compareImageUnavailable'));
      return () => {
        disposed = true;
      };
    }

    void picdogCommands
      .readCompareImages(assetId)
      .then((nextPayload) => {
        if (disposed) return;
        setPayload(nextPayload);
        if (!nextPayload.beforeDataUrl && !nextPayload.afterDataUrl) {
          setError(t('compareImageUnavailable'));
        }
      })
      .catch(() => {
        if (!disposed) setError(t('compareImageUnavailable'));
      });

    return () => {
      disposed = true;
    };
  }, [assetId, t]);

  return (
    <main className="compare-shell">
      <ComparePanel payload={payload} error={error} t={t} onClose={() => void closeWindow()} onStartDrag={() => void startWindowDrag()} />
    </main>
  );
}

function ComparePanel({
  payload,
  error,
  t,
  onClose,
  onStartDrag
}: {
  payload: CompareImagePayload | null;
  error: string;
  t: (key: I18nKey) => string;
  onClose: () => void;
  onStartDrag: () => void;
}) {
  const [split, setSplit] = useState(50);
  const [beforeFailed, setBeforeFailed] = useState(false);
  const [afterFailed, setAfterFailed] = useState(false);

  useEffect(() => {
    setBeforeFailed(false);
    setAfterFailed(false);
    setSplit(50);
  }, [payload?.assetId]);

  const dimensions = payload?.width && payload.height ? `${payload.width}×${payload.height}` : payload?.fileName ?? '—';
  const beforeVisible = Boolean(payload?.beforeDataUrl && !beforeFailed);
  const afterVisible = Boolean(payload?.afterDataUrl && !afterFailed);

  return (
    <section className="compare-window-panel">
      <header className="compare-window-titlebar" onMouseDown={onStartDrag}>
        <div className="traffic-lights mini" aria-hidden="true">
          <span className="red" />
          <span className="yellow" />
          <span className="green" />
        </div>
        <strong>{t('imageCompare')}</strong>
        <button type="button" className="close-button" onMouseDown={(event) => event.stopPropagation()} onClick={onClose} aria-label="Close">×</button>
      </header>

      <div className="compare-window-toolbar">
        <span>{dimensions}</span>
        <strong>{typeof payload?.savedRatio === 'number' ? `−${payload.savedRatio}%` : '—'}</strong>
      </div>

      <div className="compare-window-stage">
        {beforeVisible ? (
          <img className="compare-image before" src={payload?.beforeDataUrl} alt="" onError={() => setBeforeFailed(true)} />
        ) : (
          <div className="compare-placeholder">{error || t('beforeCompression')}</div>
        )}

        {afterVisible ? (
          <img
            className="compare-image after"
            src={payload?.afterDataUrl}
            alt=""
            style={{ clipPath: `inset(0 0 0 ${split}%)` }}
            onError={() => setAfterFailed(true)}
          />
        ) : null}

        {beforeVisible && afterVisible && (
          <>
            <div className="compare-divider" style={{ left: `${split}%` }}>
              <span />
            </div>
            <input
              className="compare-range"
              type="range"
              min="0"
              max="100"
              value={split}
              aria-label={t('compareImage')}
              onChange={(event) => setSplit(Number(event.target.value))}
            />
          </>
        )}

        <div className="compare-badges">
          <span>
            {t('beforeCompression')}
            <strong>{payload ? formatBytes(payload.sourceSize) : '—'}</strong>
          </span>
          <span>
            {t('afterCompression')}
            <strong>{payload?.outputSize ? formatBytes(payload.outputSize) : '—'}</strong>
          </span>
        </div>
      </div>
    </section>
  );
}
