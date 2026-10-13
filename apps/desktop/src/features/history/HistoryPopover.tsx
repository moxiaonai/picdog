import type { HistoryItem, HistoryQuery, Route } from '../../contracts/models';
import type { I18nKey } from '../../app/i18n';
import { formatBytes } from '../batch/batchState';

export type HistoryFilters = {
  keyword: string;
  routeId: string;
  status: HistoryQuery['status'] | '';
};

type HistoryPopoverProps = {
  open: boolean;
  items: HistoryItem[];
  routes: Route[];
  filters: HistoryFilters;
  selectedIds: string[];
  total: number;
  hasMore: boolean;
  clearLocalOutputs: boolean;
  statusMessage?: string | null;
  uploadEnabled: boolean;
  t: (key: I18nKey) => string;
  onFiltersChange: (patch: Partial<HistoryFilters>) => void;
  onToggleSelect: (itemId: string) => void;
  onToggleClearLocalOutputs: (checked: boolean) => void;
  onCopy: (item: HistoryItem) => void | Promise<void>;
  onReveal: (item: HistoryItem) => void | Promise<void>;
  onExport: () => void | Promise<void>;
  onClear: () => void | Promise<void>;
  onLoadMore: () => void | Promise<void>;
  onClose: () => void;
};

const statusOptions: Array<{ value: HistoryFilters['status']; label: I18nKey }> = [
  { value: '', label: 'allStatuses' },
  { value: 'uploaded', label: 'statusUploaded' },
  { value: 'compressed', label: 'statusCompressed' },
  { value: 'failed', label: 'statusFailed' }
];

export function HistoryPopover({
  open,
  items,
  routes,
  filters,
  selectedIds,
  total,
  hasMore,
  clearLocalOutputs,
  statusMessage,
  uploadEnabled,
  t,
  onFiltersChange,
  onToggleSelect,
  onToggleClearLocalOutputs,
  onCopy,
  onReveal,
  onExport,
  onClear,
  onLoadMore,
  onClose
}: HistoryPopoverProps) {
  if (!open) return null;

  const selectedCount = selectedIds.length;
  const clearLabel = selectedCount > 0 ? t('clearSelectedHistory') : t('clearHistory');

  return (
    <section className="popover history-popover" aria-label={t('history')}>
      <div className="popover-head">
        <strong>{t('history')}</strong>
        <div className="popover-actions">
          <button className="soft-button compact-primary" type="button" disabled={items.length === 0} onClick={onExport}>
            {t('exportHistory')}
          </button>
          <button className="ghost-danger" type="button" disabled={items.length === 0 && selectedCount === 0} onClick={onClear}>
            {clearLabel}
          </button>
          <button className="close-button" type="button" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
      </div>

      <div className="popover-body">
        <div className="history-filters">
          <input
            value={filters.keyword}
            placeholder={t('historySearch')}
            onChange={(event) => onFiltersChange({ keyword: event.currentTarget.value })}
          />
          <div className="history-filter-row">
            <select value={filters.routeId} onChange={(event) => onFiltersChange({ routeId: event.currentTarget.value })}>
              <option value="">{t('allTargets')}</option>
              {routes.map((route) => (
                <option value={route.id} key={route.id}>
                  {route.name}
                </option>
              ))}
            </select>
            <select
              value={filters.status}
              onChange={(event) => onFiltersChange({ status: event.currentTarget.value as HistoryFilters['status'] })}
            >
              {statusOptions.map((option) => (
                <option value={option.value} key={option.value || 'all'}>
                  {t(option.label)}
                </option>
              ))}
            </select>
          </div>
          <label className="history-clear-option">
            <input
              type="checkbox"
              checked={clearLocalOutputs}
              onChange={(event) => onToggleClearLocalOutputs(event.currentTarget.checked)}
            />
            <span>{t('deleteLocalOutputs')}</span>
          </label>
        </div>

        <div className="history-list">
          {items.length === 0 ? (
            <div className="history-empty">{t('emptyHistory')}</div>
          ) : (
            items.map((item) => (
              <article className="history-item" key={item.id}>
                <label className="history-check">
                  <input
                    type="checkbox"
                    checked={selectedIds.includes(item.id)}
                    onChange={() => onToggleSelect(item.id)}
                  />
                  <span />
                </label>
                <div className="history-main">
                  <strong>{item.fileName}</strong>
                  <small>
                    {item.routeName ?? t('localOnly')} · {item.savedRatio ? '−' + item.savedRatio + '%' : formatBytes(item.sourceSize)}
                  </small>
                </div>
                <div className="history-actions">
                  <button type="button" disabled={!item.outputPath} onClick={() => onReveal(item)}>
                    {t('revealOutput')}
                  </button>
                  {uploadEnabled && (
                    <button type="button" disabled={!item.canCopy} onClick={() => onCopy(item)}>
                      {t('copyLink')}
                    </button>
                  )}
                </div>
              </article>
            ))
          )}
        </div>

        <div className="history-footer">
          <small>
            {statusMessage || (selectedCount > 0
              ? t('selectedHistoryCount').replace('{count}', String(selectedCount))
              : String(items.length) + ' / ' + String(total))}
          </small>
          {hasMore && (
            <button className="soft-button compact-primary" type="button" onClick={onLoadMore}>
              {t('loadMore')}
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
