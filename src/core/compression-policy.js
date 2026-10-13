const FORMAT_ALIASES = new Map([
  ['jpg', 'jpeg'],
  ['jpeg', 'jpeg'],
  ['png', 'png'],
  ['gif', 'gif'],
  ['webp', 'webp'],
  ['svg', 'svg'],
  ['avif', 'avif'],
  ['heic', 'heic'],
  ['heif', 'heic'],
]);

const EXTENSION_FORMATS = new Map([
  ['jpg', 'jpeg'],
  ['jpeg', 'jpeg'],
  ['png', 'png'],
  ['gif', 'gif'],
  ['webp', 'webp'],
  ['svg', 'svg'],
  ['avif', 'avif'],
  ['heic', 'heic'],
  ['heif', 'heic'],
]);

export const compressionModes = Object.freeze({
  balanced: {
    label: 'Balanced',
    quality: 78,
    description: '体积和画质均衡，默认保持原格式。',
  },
  tiny: {
    label: 'Tiny',
    quality: 66,
    outputFormat: 'webp',
    description: '更激进地减小体积，适合移动端和弱网。',
  },
  lossless: {
    label: 'Lossless',
    quality: 100,
    outputFormat: 'original',
    description: '尽量保持像素信息，适合截图、图标和设计切图。',
  },
  retina: {
    label: 'Retina',
    quality: 82,
    outputFormat: 'original',
    description: '保留高清细节，适合作品集和摄影图。',
  },
  extreme_avif: {
    label: 'Extreme AVIF',
    quality: 48,
    outputFormat: 'avif',
    description: '转 AVIF，使用 libavif + AOM 追求极致体积。',
  },
});

export const defaultCompressionOptions = Object.freeze({
  mode: 'balanced',
  quality: 78,
  outputFormat: 'original',
  stripMetadata: true,
  preserveColorProfile: true,
  keepOriginal: true,
  fallbackWhenLarger: true,
});

const encoderPlans = {
  jpeg: {
    encoder: 'mozjpeg',
    chain: ['mozjpeg'],
    params: {
      balanced: { quality: [76, 82], progressive: true },
      tiny: { quality: [68, 74], progressive: true },
      lossless: { optimizeCoding: true, progressive: true },
      retina: { quality: [84, 90], progressive: true },
    },
  },
  png: {
    encoder: 'pngquant+oxipng',
    chain: ['pngquant', 'oxipng'],
    params: {
      balanced: { pngquantQuality: [65, 85], oxipngLevel: 3 },
      tiny: { pngquantQuality: [45, 75], oxipngLevel: 4 },
      lossless: { chain: ['oxipng'], oxipngLevel: 4 },
      retina: { pngquantQuality: [80, 95], oxipngLevel: 3 },
    },
  },
  gif: {
    encoder: 'gifsicle',
    chain: ['gifsicle'],
    params: {
      balanced: { optimizeLevel: 2 },
      tiny: { optimizeLevel: 3, allowColorReduction: true },
      lossless: { optimizeLevel: 2 },
      retina: { optimizeLevel: 2 },
    },
  },
  webp: {
    encoder: 'libwebp',
    chain: ['cwebp'],
    params: {
      balanced: { quality: [75, 82] },
      tiny: { quality: [65, 75] },
      lossless: { lossless: true },
      retina: { quality: [84, 90] },
    },
  },
  svg: {
    encoder: 'svgo',
    chain: ['svgo'],
    params: {
      balanced: { preset: 'default' },
      tiny: { preset: 'aggressive' },
      lossless: { preset: 'safe' },
      retina: { preset: 'safe' },
    },
  },
  avif: {
    encoder: 'libavif+aom',
    chain: ['libavif', 'aom'],
    params: {
      balanced: { quantizer: [28, 34], speed: [5, 7] },
      tiny: { quantizer: [34, 42], speed: [6, 8] },
      lossless: { lossless: true },
      retina: { quantizer: [20, 28], speed: [4, 6] },
      extreme_avif: { quantizer: [30, 38], speed: [5, 7] },
    },
  },
};

export function detectFormat(input) {
  if (!input) return 'unknown';

  const mime = String(input.mime || input.type || '').toLowerCase();
  if (mime.includes('jpeg')) return 'jpeg';
  if (mime.includes('png')) return 'png';
  if (mime.includes('gif')) return 'gif';
  if (mime.includes('webp')) return 'webp';
  if (mime.includes('svg')) return 'svg';
  if (mime.includes('avif')) return 'avif';
  if (mime.includes('heic') || mime.includes('heif')) return 'heic';

  const explicit = String(input.format || '').toLowerCase().replace(/^\./, '');
  if (FORMAT_ALIASES.has(explicit)) return FORMAT_ALIASES.get(explicit);

  const fileName = String(input.name || input.path || '');
  const extension = fileName.includes('.') ? fileName.split('.').pop().toLowerCase() : '';
  return EXTENSION_FORMATS.get(extension) || 'unknown';
}

export function normalizeCompressionOptions(options = {}) {
  const requestedMode = options.mode ?? defaultCompressionOptions.mode;
  const mode = compressionModes[requestedMode] ? requestedMode : defaultCompressionOptions.mode;
  const preset = compressionModes[mode] || compressionModes[defaultCompressionOptions.mode];
  const merged = { ...defaultCompressionOptions, ...preset, ...options };
  const quality = Number.isFinite(options.quality)
    ? Math.min(100, Math.max(1, Math.round(options.quality)))
    : preset.quality;
  const outputFormat = normalizeOutputFormat(
    Object.prototype.hasOwnProperty.call(options, 'outputFormat')
      ? options.outputFormat
      : preset.outputFormat || defaultCompressionOptions.outputFormat
  );

  return {
    ...merged,
    mode,
    quality,
    outputFormat,
    stripMetadata: merged.stripMetadata !== false,
    preserveColorProfile: merged.preserveColorProfile !== false,
    keepOriginal: merged.keepOriginal !== false,
    fallbackWhenLarger: merged.fallbackWhenLarger !== false,
  };
}

export function normalizeOutputFormat(outputFormat) {
  const raw = String(outputFormat || 'original').toLowerCase().replace(/^\./, '');
  if (raw === 'original') return 'original';
  return FORMAT_ALIASES.get(raw) || 'original';
}

export function resolveCompressionPlan(input, options = {}) {
  const normalized = normalizeCompressionOptions(options);
  const sourceFormat = detectFormat(input);
  const targetFormat = resolveTargetFormat(sourceFormat, normalized, input);

  if (targetFormat === 'unsupported') {
    return {
      supported: false,
      reason: buildUnsupportedReason(sourceFormat),
      sourceFormat,
      targetFormat: sourceFormat,
      mode: normalized.mode,
      keepOriginal: true,
      fallbackWhenLarger: true,
      steps: [],
      warnings: ['unsupported_format'],
    };
  }

  const plan = encoderPlans[targetFormat];
  const params = resolveParams(plan, normalized.mode, normalized.quality);
  const warnings = collectWarnings(sourceFormat, targetFormat, input, normalized);

  return {
    supported: true,
    sourceFormat,
    targetFormat,
    mode: normalized.mode,
    quality: normalized.quality,
    encoder: plan.encoder,
    steps: params.chain || plan.chain,
    params,
    stripMetadata: normalized.stripMetadata,
    preserveColorProfile: normalized.preserveColorProfile,
    keepOriginal: normalized.keepOriginal,
    fallbackWhenLarger: normalized.fallbackWhenLarger,
    warnings,
  };
}

function resolveTargetFormat(sourceFormat, options, input = {}) {
  if (!encoderPlans[sourceFormat] && sourceFormat !== 'heic') return 'unsupported';

  if (options.mode === 'extreme_avif') {
    if (input.animated || sourceFormat === 'gif' || sourceFormat === 'svg') {
      return sourceFormat === 'svg' ? 'svg' : 'webp';
    }
    return 'avif';
  }

  if (options.outputFormat === 'original') {
    return sourceFormat === 'heic' ? 'jpeg' : sourceFormat;
  }

  if (options.outputFormat === 'avif') {
    if (input.animated || sourceFormat === 'gif' || sourceFormat === 'svg') {
      return sourceFormat === 'svg' ? 'svg' : 'webp';
    }
    return 'avif';
  }

  if (options.outputFormat === 'webp') {
    if (sourceFormat === 'svg') return 'svg';
    return 'webp';
  }

  if (options.outputFormat === 'jpg' || options.outputFormat === 'jpeg') return 'jpeg';
  if (options.outputFormat === 'png') return 'png';

  return sourceFormat;
}

function resolveParams(plan, mode, quality) {
  const params = plan.params[mode] || plan.params.balanced || {};
  return {
    ...params,
    quality,
  };
}

function collectWarnings(sourceFormat, targetFormat, input, options) {
  const warnings = [];

  if (targetFormat === 'avif') {
    warnings.push('avif_slower_encoding');
  }

  if (sourceFormat === 'png' && targetFormat === 'jpeg' && input.hasAlpha) {
    warnings.push('alpha_channel_will_be_flattened');
  }

  if (options.stripMetadata) {
    warnings.push('metadata_will_be_stripped');
  }

  return warnings;
}

function buildUnsupportedReason(sourceFormat) {
  if (sourceFormat === 'unknown') return '无法识别图片格式。';
  return `暂不支持 ${sourceFormat.toUpperCase()} 压缩。`;
}

export function shouldKeepCompressedOutput(result) {
  if (!result || result.sourceSize == null || result.outputSize == null) return true;
  if (result.fallbackWhenLarger === false) return true;
  return Number(result.outputSize) <= Number(result.sourceSize);
}

