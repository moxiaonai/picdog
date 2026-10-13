import { useMemo, useState } from 'react';
import type {
  AppSettings,
  CompressionMode,
  CompressionToolStatus,
  CopyFormat,
  Locale,
  OutputFormat
} from '../../contracts/models';
import { compressionPresetForMode, compressionPresetOrder } from '../../contracts/compressionPresets';
import type { I18nKey } from '../../app/i18n';
import { writeClipboardText } from '../../app/clipboard';

type SettingsPopoverProps = {
  open: boolean;
  settings: AppSettings;
  compressionTools: CompressionToolStatus[];
  t: (key: I18nKey) => string;
  onLocaleChange: (locale: Locale) => void;
  onCopyFormatChange: (format: CopyFormat) => void;
  onSettingsChange: (patch: Partial<AppSettings>) => void;
  onRefreshCompressionTools: () => void | Promise<void>;
  onSelectDirectory: (title?: string) => Promise<string | null>;
  onClose: () => void;
};

const locales: Locale[] = ['zh-CN', 'en-US', 'ja-JP'];

const compressionModeLabels: Record<CompressionMode, I18nKey> = {
  balanced: 'balancedMode',
  tiny: 'tinyMode',
  lossless: 'losslessMode',
  retina: 'retinaMode',
  extreme_avif: 'extremeAvifMode',
  custom: 'balancedMode'
};

const outputFormats: Array<{ value: OutputFormat; label?: I18nKey; name?: string }> = [
  { value: 'original', label: 'originalFormat' },
  { value: 'webp', name: 'WebP' },
  { value: 'avif', name: 'AVIF' },
  { value: 'jpg', name: 'JPG' },
  { value: 'png', name: 'PNG' }
];

const compressionToolPurposeLabels: Record<string, I18nKey> = {
  'builtin-image': 'toolPurposeBuiltinImage',
  cjpeg: 'toolPurposeCjpeg',
  pngquant: 'toolPurposePngquant',
  oxipng: 'toolPurposeOxipng',
  gifsicle: 'toolPurposeGifsicle',
  cwebp: 'toolPurposeCwebp',
  gif2webp: 'toolPurposeGif2webp',
  avifenc: 'toolPurposeAvifenc',
  svgo: 'toolPurposeSvgo'
};

function compressionToolPurpose(tool: CompressionToolStatus, t: (key: I18nKey) => string) {
  const key = compressionToolPurposeLabels[tool.name];
  return key ? t(key) : tool.purpose;
}

function clampQuality(value: number) {
  return Math.min(100, Math.max(1, value));
}

function optionalDimension(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return undefined;

  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed) || parsed <= 0) return undefined;

  return Math.min(20000, Math.round(parsed));
}

function formatLabel(format: { label?: I18nKey; name?: string }, t: (key: I18nKey) => string) {
  return format.label ? t(format.label) : format.name ?? '';
}

function installCommandForMissingTools(tools: CompressionToolStatus[]) {
  const brewPackages = new Set<string>();
  let needsSvgo = false;

  for (const tool of tools) {
    if (tool.available) continue;

    if (tool.name === 'cjpeg') brewPackages.add('mozjpeg');
    if (tool.name === 'pngquant') brewPackages.add('pngquant');
    if (tool.name === 'oxipng') brewPackages.add('oxipng');
    if (tool.name === 'gifsicle') brewPackages.add('gifsicle');
    if (tool.name === 'cwebp' || tool.name === 'gif2webp') brewPackages.add('webp');
    if (tool.name === 'avifenc') brewPackages.add('libavif');
    if (tool.name === 'svgo') needsSvgo = true;
  }

  const commands = [];
  if (brewPackages.size > 0) {
    commands.push('brew install ' + Array.from(brewPackages).join(' '));
  }
  if (needsSvgo) {
    commands.push('npm install -g svgo');
  }

  return commands.join(' && ');
}

export function SettingsPopover({
  open,
  settings,
  compressionTools,
  t,
  onLocaleChange,
  onCopyFormatChange,
  onSettingsChange,
  onRefreshCompressionTools,
  onSelectDirectory,
  onClose
}: SettingsPopoverProps) {
  const [installCommandCopied, setInstallCommandCopied] = useState(false);
  const missingTools = useMemo(
    () => compressionTools.filter((tool) => !tool.available),
    [compressionTools]
  );
  const installCommand = useMemo(
    () => installCommandForMissingTools(missingTools),
    [missingTools]
  );

  if (!open) return null;

  const applyCompressionMode = (mode: CompressionMode) => {
    const preset = compressionPresetForMode(mode);

    onSettingsChange({
      compressionMode: mode,
      compressionQuality: preset.quality,
      outputFormat: preset.outputFormat
    });
  };

  const copyInstallCommand = async () => {
    if (!installCommand) return;

    await writeClipboardText(installCommand);
    setInstallCommandCopied(true);
    window.setTimeout(() => setInstallCommandCopied(false), 1600);
  };

  const chooseEncoderDirectory = async () => {
    const path = await onSelectDirectory(t('chooseEncoderDirectoryTitle'));
    if (path) {
      onSettingsChange({ encoderDirectory: path });
    }
  };

  return (
    <section className="popover settings-popover" aria-label={t('settings')}>
      <div className="popover-head">
        <strong>{t('settings')}</strong>
        <button className="close-button" type="button" onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>

      <div className="popover-body">
        <label className="settings-row">
          <span>
            <strong>{t('language')}</strong>
            <small>中文 / English / 日本語</small>
          </span>
          <div className="segmented" role="group" aria-label={t('language')}>
            {locales.map((locale) => (
              <button
                key={locale}
                type="button"
                className={settings.locale === locale ? 'active' : ''}
                onClick={() => onLocaleChange(locale)}
              >
                {locale === 'zh-CN' ? '中文' : locale === 'en-US' ? 'EN' : '日本語'}
              </button>
            ))}
          </div>
        </label>

        <section className="setting-section compression-card" aria-label={t('compressionDefaults')}>
          <div className="section-title">
            <span>
              <strong>{t('compressionDefaults')}</strong>
              <small>{t('compressionDefaultsDesc')}</small>
            </span>
            <select
              className="select-control"
              value={settings.outputFormat}
              onChange={(event) => onSettingsChange({ outputFormat: event.target.value as OutputFormat })}
              aria-label={t('outputFormat')}
            >
              {outputFormats.map((format) => (
                <option key={format.value} value={format.value}>
                  {formatLabel(format, t)}
                </option>
              ))}
            </select>
          </div>

          <div className="compression-controls">
            <div className="segmented mode-segmented" role="group" aria-label={t('compressionMode')}>
              {compressionPresetOrder.map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className={settings.compressionMode === mode ? 'active' : ''}
                  onClick={() => applyCompressionMode(mode)}
                >
                  {t(compressionModeLabels[mode])}
                </button>
              ))}
            </div>

            <label className="range-control">
              <span>{t('compressionQuality')}</span>
              <input
                type="range"
                min="1"
                max="100"
                value={settings.compressionQuality}
                onChange={(event) => onSettingsChange({ compressionQuality: clampQuality(Number(event.target.value)) })}
              />
              <b>{settings.compressionQuality}</b>
            </label>
          </div>

          <div className="dimension-controls">
            <label>
              <span>{t('maxWidth')}</span>
              <input
                type="number"
                min="1"
                max="20000"
                placeholder="Auto"
                value={settings.maxWidth ?? ''}
                onChange={(event) => onSettingsChange({ maxWidth: optionalDimension(event.target.value) })}
              />
            </label>
            <label>
              <span>{t('maxHeight')}</span>
              <input
                type="number"
                min="1"
                max="20000"
                placeholder="Auto"
                value={settings.maxHeight ?? ''}
                onChange={(event) => onSettingsChange({ maxHeight: optionalDimension(event.target.value) })}
              />
            </label>
          </div>

          <div className="toggle-row">
            <label>
              <input
                type="checkbox"
                checked={settings.stripMetadata}
                onChange={(event) => onSettingsChange({ stripMetadata: event.target.checked })}
              />
              <span>{t('stripMetadata')}</span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={settings.preserveColorProfile}
                onChange={(event) => onSettingsChange({ preserveColorProfile: event.target.checked })}
              />
              <span>{t('preserveColorProfile')}</span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={settings.fallbackWhenLarger}
                onChange={(event) => onSettingsChange({ fallbackWhenLarger: event.target.checked })}
              />
              <span>{t('fallbackWhenLarger')}</span>
            </label>
          </div>

          <div className="tool-status-card">
            <div className="tool-status-head">
              <span>
                <strong>{t('compressionTools')}</strong>
                <small>{t('compressionToolsDesc')}</small>
              </span>
              <button type="button" className="soft-button" onClick={onRefreshCompressionTools}>
                {t('refresh')}
              </button>
            </div>
            <div className="tool-status-list">
              {compressionTools.map((tool) => (
                <span className={tool.available ? 'tool-ok' : 'tool-missing'} key={tool.name} title={compressionToolPurpose(tool, t)}>
                  <b>{tool.name}</b>
                  <small>{tool.available ? t('available') : t('missing')}</small>
                </span>
              ))}
            </div>
            <div className="encoder-directory-row">
              <span>
                <strong>{t('encoderDirectory')}</strong>
                <small title={settings.encoderDirectory ?? t('encoderDirectoryDesc')}>
                  {settings.encoderDirectory || t('encoderDirectoryDesc')}
                </small>
              </span>
              <div>
                {settings.encoderDirectory && (
                  <button
                    type="button"
                    className="soft-button"
                    onClick={() => onSettingsChange({ encoderDirectory: undefined })}
                  >
                    {t('clear')}
                  </button>
                )}
                <button type="button" className="soft-button" onClick={() => void chooseEncoderDirectory()}>
                  {t('chooseFolder')}
                </button>
              </div>
            </div>
            {missingTools.length > 0 && installCommand && (
              <div className="install-hint">
                <span>
                  <strong>{t('missingToolsTitle')}</strong>
                  <small>{t('missingToolsDesc')}</small>
                </span>
                <code>{installCommand}</code>
                <button type="button" className="soft-button" onClick={() => void copyInstallCommand()}>
                  {installCommandCopied ? t('installCommandCopied') : t('copyInstallCommand')}
                </button>
              </div>
            )}
          </div>
        </section>

        {settings.uploadEnabled && (
          <label className="settings-row">
            <span>
              <strong>{t('copyFormat')}</strong>
              <small>{t('copyAfterUpload')}</small>
            </span>
            <select value={settings.copyFormat} onChange={(event) => onCopyFormatChange(event.target.value as CopyFormat)}>
              <option value="markdown">{t('copyFormatMarkdown')}</option>
              <option value="url">{t('copyFormatUrl')}</option>
              <option value="html">{t('copyFormatHtml')}</option>
              <option value="custom">{t('copyFormatCustom')}</option>
            </select>
          </label>
        )}

        <label className="settings-row">
          <span>
            <strong>{t('clearDefaults')}</strong>
            <small>{t('clearDefaultsDesc')}</small>
          </span>
          <label className="settings-checkbox">
            <input
              type="checkbox"
              checked={settings.deleteLocalOutputsOnClear}
              onChange={(event) => onSettingsChange({ deleteLocalOutputsOnClear: event.target.checked })}
            />
            <span>{t('deleteLocalOutputs')}</span>
          </label>
        </label>

        <label className="settings-row">
          <span>
            <strong>{t('privacyDefaults')}</strong>
            <small>{t('hideLocalPathDesc')}</small>
          </span>
          <label className="settings-checkbox">
            <input
              type="checkbox"
              checked={settings.hideLocalPath}
              onChange={(event) => onSettingsChange({ hideLocalPath: event.target.checked })}
            />
            <span>{t('hideLocalPath')}</span>
          </label>
        </label>
      </div>
    </section>
  );
}
