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
                <button type="button" disabled={!canReveal(asset)} title={t('revealOutput')} onClick={() => onReveal(asset.id)}>
                  ⌕
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
    </section>
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

