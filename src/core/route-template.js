export function renderRoutePath(template, asset, date = new Date()) {
  const safeTemplate = template || '{yyyy}/{mm}/{hash}-{name}.{ext}';
  const ext = normalizeExt(asset.outputFormat || asset.targetFormat || asset.ext || asset.name);
  const name = slugify(stripExtension(asset.name || asset.fileName || 'image'));
  const hash = String(asset.hash || asset.id || 'asset').slice(0, 12);
  const tokens = {
    yyyy: String(date.getFullYear()),
    mm: pad2(date.getMonth() + 1),
    dd: pad2(date.getDate()),
    name,
    slug: name,
    hash,
    ext,
  };

  return safeTemplate.replace(/\{(yyyy|mm|dd|name|slug|hash|ext)\}/g, (_, key) => tokens[key]);
}

export function buildPublicUrl(publicBaseUrl, remoteKey) {
  const base = String(publicBaseUrl || '').replace(/\/+$/, '');
  const key = String(remoteKey || '').replace(/^\/+/, '');
  return base && key ? `${base}/${key}` : base || key;
}

function pad2(value) {
  return String(value).padStart(2, '0');
}

function stripExtension(fileName) {
  return String(fileName).replace(/\.[^.]+$/, '');
}

function normalizeExt(value) {
  const raw = String(value || 'jpg').toLowerCase();
  if (!raw.includes('.')) return raw.replace(/^jpeg$/, 'jpg');
  return raw.split('.').pop().replace(/^jpeg$/, 'jpg');
}

function slugify(value) {
  return String(value)
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-zA-Z0-9-_]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase() || 'image';
}

