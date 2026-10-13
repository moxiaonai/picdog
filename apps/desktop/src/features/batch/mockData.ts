import type { AppSettings, Batch, FileInput, HistoryItem, Route } from '../../contracts/models';
import { defaultCompressionOptions } from '../../contracts/compressionPresets';

function mockThumb(label: string, colorA: string, colorB: string) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="${colorA}"/><stop offset="1" stop-color="${colorB}"/></linearGradient></defs><rect width="96" height="96" rx="18" fill="url(#g)"/><circle cx="70" cy="24" r="10" fill="white" opacity=".72"/><path d="M10 76 34 50l14 14 13-18 25 30H10Z" fill="white" opacity=".78"/><text x="14" y="24" fill="white" font-family="Arial,sans-serif" font-size="12" font-weight="700">${label}</text></svg>`;
  return 'data:image/svg+xml,' + encodeURIComponent(svg);
}

const now = () => new Date().toISOString();

export const demoFiles: FileInput[] = [
  { path: '/Users/demo/Pictures/hero-cover.png', name: 'hero-cover.png', size: 3400000, mime: 'image/png', previewUrl: mockThumb('PNG', '#68a6ff', '#7fd0c8') },
  { path: '/Users/demo/Pictures/product-shot.jpg', name: 'product-shot.jpg', size: 2600000, mime: 'image/jpeg', previewUrl: mockThumb('JPG', '#f59e6f', '#f7cf73') },
  { path: '/Users/demo/Pictures/feature-loop.gif', name: 'feature-loop.gif', size: 1800000, mime: 'image/gif', previewUrl: mockThumb('GIF', '#a78bfa', '#60a5fa') },
  { path: '/Users/demo/Pictures/docs-diagram.svg', name: 'docs-diagram.svg', size: 800000, mime: 'image/svg+xml', previewUrl: mockThumb('SVG', '#34d399', '#38bdf8') }
];

export const mockRoutes: Route[] = [];

export const mockSettings: AppSettings = {
  locale: 'zh-CN',
  uploadEnabled: false,
  copyFormat: 'markdown',
  hideLocalPath: false,
  deleteLocalOutputsOnClear: false,
  encoderDirectory: undefined,
  compressionMode: defaultCompressionOptions.mode,
  compressionQuality: defaultCompressionOptions.quality,
  outputFormat: defaultCompressionOptions.outputFormat,
  stripMetadata: defaultCompressionOptions.stripMetadata,
  preserveColorProfile: defaultCompressionOptions.preserveColorProfile,
  keepOriginal: defaultCompressionOptions.keepOriginal,
  fallbackWhenLarger: defaultCompressionOptions.fallbackWhenLarger
};

export const mockHistory: HistoryItem[] = [];

export function createMockBatch(files = demoFiles): Batch {
  const batchId = 'batch-' + Date.now();
  const seen = new Set<string>();
  const uniqueFiles = files.filter((file) => {
    const key = [file.path, file.name, file.size].join('|');
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });

  return {
    id: batchId,
    status: 'queue',
    createdAt: now(),
    updatedAt: now(),
    assets: uniqueFiles.map((file, index) => ({
      id: 'asset-' + Date.now() + '-' + index,
      batchId,
      name: file.name,
      sourcePath: file.path,
      sourceSize: file.size,
      mime: file.mime,
      previewUrl: file.previewUrl,
      compressStatus: 'pending',
      uploadStatus: 'none',
      compressProgress: 0,
      uploadProgress: 0,
      selected: false
    }))
  };
}
