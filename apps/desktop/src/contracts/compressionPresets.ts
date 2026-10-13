import type { CompressionMode, CompressionOptions, OutputFormat } from './models';

type CompressionPreset = {
  quality: number;
  outputFormat: OutputFormat;
};

export const compressionPresetOrder: CompressionMode[] = [
  'balanced',
  'tiny',
  'lossless',
  'extreme_avif'
];

export const compressionPresets: Record<CompressionMode, CompressionPreset> = {
  balanced: { quality: 78, outputFormat: 'original' },
  tiny: { quality: 66, outputFormat: 'webp' },
  lossless: { quality: 100, outputFormat: 'original' },
  retina: { quality: 82, outputFormat: 'original' },
  extreme_avif: { quality: 48, outputFormat: 'avif' },
  custom: { quality: 78, outputFormat: 'original' }
};

export const defaultCompressionOptions: CompressionOptions = {
  mode: 'balanced',
  quality: compressionPresets.balanced.quality,
  outputFormat: compressionPresets.balanced.outputFormat,
  stripMetadata: true,
  preserveColorProfile: true,
  keepOriginal: true,
  fallbackWhenLarger: true
};

export function compressionPresetForMode(mode: CompressionMode): CompressionPreset {
  return compressionPresets[mode] ?? compressionPresets.balanced;
}
