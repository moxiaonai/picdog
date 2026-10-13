const builtInFormats = new Set(['url', 'markdown', 'html', 'custom']);

export function normalizeCopyFormat(format) {
  const value = String(format || 'url').toLowerCase();
  return builtInFormats.has(value) ? value : 'url';
}

export function renderCopyText(asset, format = 'url', customTemplate = '') {
  if (!asset || !asset.publicUrl) {
    throw new Error('上传完成后才能复制链接。');
  }

  const normalized = normalizeCopyFormat(format);
  const tokens = {
    url: asset.publicUrl,
    name: asset.name || asset.fileName || '',
    width: asset.width || '',
    height: asset.height || '',
    alt: asset.alt || asset.name || asset.fileName || '',
  };

  if (normalized === 'markdown') {
    return `![${tokens.alt}](${tokens.url})`;
  }

  if (normalized === 'html') {
    const size = tokens.width && tokens.height ? ` width="${tokens.width}" height="${tokens.height}"` : '';
    return `<img src="${tokens.url}" alt="${escapeHtml(tokens.alt)}"${size} />`;
  }

  if (normalized === 'custom') {
    return applyTemplate(customTemplate || '{url}', tokens);
  }

  return tokens.url;
}

export function applyTemplate(template, tokens) {
  return String(template).replace(/\{(url|name|width|height|alt)\}/g, (_, key) => String(tokens[key] ?? ''));
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

