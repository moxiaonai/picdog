import type { I18nKey } from '../../app/i18n';
import type { AssetState, BatchState } from '../../contracts/models';

type ActionBarProps = {
  state: BatchState;
  assets: AssetState[];
  uploadEnabled: boolean;
  error?: string | null;
  canClear: boolean;
  canCancel: boolean;
  canCopyAll: boolean;
  canSelectAll: boolean;
  canPrimary: boolean;
  primaryActionKind: 'choose' | 'compress' | 'retryCompression' | 'uploadSelected' | 'busy';
  t: (key: I18nKey) => string;
  onPrimary: () => void;
  onCancel: () => void;
  onCopyAll: () => void;
  onClear: () => void;
  onSelectAll: () => void;
};

const primaryLabel: Record<ActionBarProps['primaryActionKind'], I18nKey> = {
  choose: 'choose',
  compress: 'compress',
  retryCompression: 'retryCompression',
  uploadSelected: 'uploadSelected',
  busy: 'compress'
};

export function ActionBar({
  state,
  assets,
  uploadEnabled,
  error,
  canClear,
  canCancel,
  canCopyAll,
  canSelectAll,
  canPrimary,
  primaryActionKind,
  t,
  onPrimary,
  onCancel,
  onCopyAll,
  onClear,
  onSelectAll
}: ActionBarProps) {
  const busy = state === 'compressing' || state === 'uploading' || state === 'cancelling';
  const compressionEffect = batchCompressionEffect(assets);
  const showPrimary = uploadEnabled || (state !== 'compressed' && primaryActionKind !== 'uploadSelected');

  return (
    <footer className="actionbar" title={error ?? undefined}>
      <div className="batch-stats">
        <span className="batch-stat">
          <small>{t('imageCount')}</small>
          <strong>{assets.length}</strong>
        </span>
        <span className="batch-stat">
          <small>{t('compressionEffect')}</small>
          <strong>{formatCompressionEffect(compressionEffect)}</strong>
        </span>
      </div>
      <div className="action-group">
        {canCancel && (
          <button className="soft-button danger-soft" type="button" onClick={onCancel}>
            {t('cancel')}
          </button>
        )}
        {canClear && (
          <button className="soft-button danger-soft" type="button" onClick={onClear}>
            {t('clear')}
          </button>
        )}
        {uploadEnabled && canCopyAll && (
          <button className="soft-button" type="button" onClick={onCopyAll}>
            {t('copyAllLinks')}
          </button>
        )}
        {uploadEnabled && canSelectAll && (
          <button className="soft-button" type="button" onClick={onSelectAll}>
            {t('selectAll')}
          </button>
        )}
        {showPrimary && (
          <button className="primary-button" type="button" disabled={busy || !canPrimary} onClick={onPrimary}>
            {t(primaryLabel[primaryActionKind])}
          </button>
        )}
      </div>
    </footer>
  );
}

function batchCompressionEffect(assets: AssetState[]) {
  const compressedAssets = assets.filter((asset) =>
    asset.compressStatus === 'done' &&
    asset.sourceSize > 0 &&
    (typeof asset.outputSize === 'number' || typeof asset.savedRatio === 'number')
  );

  if (compressedAssets.length === 0) return null;

  const sourceSize = compressedAssets.reduce((sum, asset) => sum + asset.sourceSize, 0);
  const outputSize = compressedAssets.reduce((sum, asset) => {
    if (typeof asset.outputSize === 'number') return sum + asset.outputSize;
    return sum + asset.sourceSize * (1 - (asset.savedRatio ?? 0) / 100);
  }, 0);

  return Math.round((1 - outputSize / sourceSize) * 100);
}

function formatCompressionEffect(effect: number | null) {
  if (effect === null) return '—';
  if (effect > 0) return `−${effect}%`;
  if (effect < 0) return `+${Math.abs(effect)}%`;
  return '0%';
}

