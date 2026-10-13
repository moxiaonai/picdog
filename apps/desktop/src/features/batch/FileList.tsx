import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { AssetState } from '../../contracts/models';
import type { I18nKey } from '../../app/i18n';
import { picdogCommands } from '../../contracts/commands';
import { canCopy, canReveal, canUpload, formatBytes } from './batchState';

type FileListProps = {
  assets: AssetState[];
  uploadEnabled: boolean;
  t: (key: I18nKey) => string;
  onToggle: (assetId: string) => void;
  onUploadOne: (assetId: string) => void;
  onCopy: (assetId: string) => void;
  onReveal: (assetId: string) => void;
};

export function FileList({
  assets,
  uploadEnabled,
  t,
  onToggle,
  onUploadOne,
  onCopy,
  onReveal
}: FileListProps) {
  const [compareAsset, setCompareAsset] = useState<AssetState | null>(null);

  if (assets.length === 0) return null;

  return (
    <section className="file-board">
      <div className="file-list">
        {assets.map((asset) => {
          const isCompressed = asset.compressStatus === 'done' && asset.compressProgress >= 100;
          const isUploaded = uploadEnabled && asset.uploadStatus === 'done' && asset.uploadProgress >= 100;

          return (
            <article
              className={isUploaded ? 'file-card is-uploaded' : uploadEnabled ? 'file-card' : 'file-card upload-disabled'}
              key={asset.id}
            >
              {uploadEnabled && (
                <label className="check-wrap">
                  <input
                    type="checkbox"
                    checked={asset.selected}
                    disabled={!canUpload(asset)}
                    onChange={() => onToggle(asset.id)}
                  />
                  <span />
                </label>
              )}

              <ImageThumb asset={asset} />

              <div className="file-main">
                <div className="file-top">
                  <strong>{asset.name}</strong>
                  <small>
                    {formatBytes(asset.sourceSize)}
                    {asset.outputSize ? ' → ' + formatBytes(asset.outputSize) + ' · −' + asset.savedRatio + '%' : ''}
                  </small>
                </div>

                <div className="progress-pair">
                  <ProgressLine label={t('compress')} value={asset.compressProgress} done={isCompressed} />
                  {uploadEnabled && <ProgressLine label="CDN" value={asset.uploadProgress} done={isUploaded} />}
                </div>
              </div>

              <div className="icon-actions">
                {uploadEnabled && (
                  <button type="button" disabled={!canUpload(asset)} title={t('uploadOne')} onClick={() => onUploadOne(asset.id)}>
                    ⇧
                  </button>
                )}
                <button type="button" disabled={!canCompare(asset)} title={t('compareImage')} onClick={() => setCompareAsset(asset)}>
                  <CompareIcon />
                </button>
                <button type="button" disabled={!canReveal(asset)} title={t('revealOutput')} onClick={() => onReveal(asset.id)}>
                  <FolderIcon />
                </button>
                {uploadEnabled && (
                  <button type="button" disabled={!canCopy(asset)} title={canCopy(asset) ? t('copyLink') : t('copyAfterUpload')} onClick={() => onCopy(asset.id)}>
                    ⛓
                  </button>
                )}
              </div>
            </article>
          );
        })}
      </div>
      {compareAsset && (
        <CompareDialog
          asset={compareAsset}
          t={t}
          onClose={() => setCompareAsset(null)}
        />
      )}
    </section>
  );
}

function canCompare(asset: AssetState) {
  return canReveal(asset) && Boolean(asset.sourcePath && asset.outputPath);
}

function CompareDialog({ asset, t, onClose }: { asset: AssetState; t: (key: I18nKey) => string; onClose: () => void }) {
  const [split, setSplit] = useState(50);
  const [sourceFailed, setSourceFailed] = useState(false);
  const [outputFailed, setOutputFailed] = useState(false);
  const sourceSrc = fullImageUrl(asset.sourcePath, asset.previewUrl);
  const outputSrc = fullImageUrl(asset.outputPath);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  useEffect(() => {
    setSourceFailed(false);
    setOutputFailed(false);
    setSplit(50);
  }, [asset.id]);

  return (
    <div className="compare-backdrop" role="presentation" onMouseDown={onClose}>
      <div
        className="compare-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t('compareImage')}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="compare-titlebar">
          <div className="traffic-lights mini" aria-hidden="true">
            <span className="red" />
            <span className="yellow" />
            <span className="green" />
          </div>
          <strong>{t('imageCompare')}</strong>
          <button type="button" className="close-button" onClick={onClose} aria-label="Close">×</button>
        </div>

        <div className="compare-toolbar">
          <span>{asset.width && asset.height ? `${asset.width}×${asset.height}` : asset.name}</span>
          <strong>{typeof asset.savedRatio === 'number' ? `−${asset.savedRatio}%` : '—'}</strong>
        </div>

        <div className="compare-stage">
          {sourceSrc && !sourceFailed ? (
            <img className="compare-image before" src={sourceSrc} alt="" onError={() => setSourceFailed(true)} />
          ) : (
            <div className="compare-placeholder">{t('beforeCompression')}</div>
          )}

          {outputSrc && !outputFailed ? (
            <img
              className="compare-image after"
              src={outputSrc}
              alt=""
              style={{ clipPath: `inset(0 0 0 ${split}%)` }}
              onError={() => setOutputFailed(true)}
            />
          ) : null}

          <div className="compare-divider" style={{ left: `${split}%` }}>
            <span />
          </div>

          <div className="compare-badges">
            <span>
              {t('beforeCompression')}
              <strong>{formatBytes(asset.sourceSize)}</strong>
            </span>
            <span>
              {t('afterCompression')}
              <strong>{asset.outputSize ? formatBytes(asset.outputSize) : '—'}</strong>
            </span>
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
        </div>
      </div>
    </div>
  );
}

function ImageThumb({ asset }: { asset: AssetState }) {
  const [previewSrc, setPreviewSrc] = useState('');
  const [failed, setFailed] = useState(false);
  const extension = asset.name.split('.').pop()?.slice(0, 3).toUpperCase() || 'IMG';

  useEffect(() => {
    let disposed = false;
    setFailed(false);

    const localPreview = thumbnailUrlForAsset(asset);
    if (localPreview) {
      setPreviewSrc(localPreview);
      return () => {
        disposed = true;
      };
    }

    if (!asset.sourcePath || !hasTauriRuntime()) {
      setPreviewSrc('');
      return () => {
        disposed = true;
      };
    }

    void picdogCommands
      .readImagePreview(asset.sourcePath)
      .then((dataUrl) => {
        if (disposed) return;
        setPreviewSrc(dataUrl || convertSourcePath(asset.sourcePath));
      })
      .catch(() => {
        if (disposed) return;
        setPreviewSrc(convertSourcePath(asset.sourcePath));
      });

    return () => {
      disposed = true;
    };
  }, [asset.previewUrl, asset.sourcePath]);

  return (
    <div className={previewSrc && !failed ? 'thumb has-preview' : 'thumb'}>
      {previewSrc && !failed ? (
        <img src={previewSrc} alt="" loading="lazy" onError={() => setFailed(true)} />
      ) : (
        <span>{extension}</span>
      )}
    </div>
  );
}

function fullImageUrl(path?: string, fallbackPreviewUrl?: string) {
  if (!path) return fallbackPreviewUrl ?? '';

  if (
    path.startsWith('blob:') ||
    path.startsWith('data:') ||
    path.startsWith('http')
  ) {
    return path;
  }

  if (!hasTauriRuntime()) return fallbackPreviewUrl ?? '';

  try {
    return convertFileSrc(path);
  } catch {
    return fallbackPreviewUrl ?? '';
  }
}

function CompareIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <rect x="3" y="4" width="14" height="12" rx="2.5" />
      <path d="M10 4v12" />
      <path d="M6.5 7.5h1.5M12 7.5h1.5M6.5 12.5h1.5M12 12.5h1.5" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="M2.8 6.2h5.1l1.3 1.6h8v6.7a2 2 0 0 1-2 2H4.8a2 2 0 0 1-2-2V6.2Z" />
      <path d="M2.8 8h14.4" />
    </svg>
  );
}

function thumbnailUrlForAsset(asset: AssetState) {
  if (asset.previewUrl) return asset.previewUrl;
  if (!asset.sourcePath) return '';

  if (
    asset.sourcePath.startsWith('blob:') ||
    asset.sourcePath.startsWith('data:') ||
    asset.sourcePath.startsWith('http')
  ) {
    return asset.sourcePath;
  }

  return '';
}

function convertSourcePath(path: string) {
  if (!path || !hasTauriRuntime()) return '';

  try {
    return convertFileSrc(path);
  } catch {
    return '';
  }
}


function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function ProgressLine({ label, value, done = false }: { label: string; value: number; done?: boolean }) {
  const progressValue = done ? 100 : value;

  return (
    <div className={done ? 'progress-line is-success' : 'progress-line'}>
      <span>{label}</span>
      <div>
        <i style={{ width: progressValue + '%' }} />
      </div>
      <span className="progress-status-icon">
        {done ? <em className="success-icon" title="success" aria-label="success">✓</em> : null}
      </span>
      <b>{progressValue}%</b>
    </div>
  );
}

