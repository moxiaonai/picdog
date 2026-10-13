const dictionaries = {
  zh: {
    appSubtitle:'图片压缩神器，帮你的图片减减肥',
    settingsTitle:'设置',
    settingsDesc:'语言、压缩、复制与隐私',
    cdnTitle:'CDN 配置',
    cdnDesc:'上传目标、CDN 域名和存储路径',
    uploadFeatureTitle:'上传功能',
    uploadFeatureDesc:'开启后显示 CDN 上传与链接操作',
    languageTitle:'界面语言',
    languageDesc:'默认中文，可随时切换',
    targetTitle:'上传目标',
    targetDesc:'压缩后手动上传到当前目标',
    cdnDomainTitle:'CDN 域名',
    cdnDomainDesc:'复制链接时使用的访问域名',
    cdnPathTitle:'存储路径',
    cdnPathDesc:'支持日期和文件名模板',
    formatTitle:'复制格式',
    formatDesc:'上传完成后复制 CDN 链接',
    historyTitle:'历史记录',
    historyDesc:'最近处理的图片和链接',
    historyButton:'历史记录',
    historyPrivacy:'只保存结果、状态和链接，不保存上传密钥。',
    historyCountUnit:'条记录',
    historyEmpty:'暂无历史记录',
    historyCompressed:'已压缩',
    historyUploaded:'已上传',
    clearHistory:'清除',
    viewHistory:'查看',
    clearHistoryDone:'历史记录已清除',
    emptyTitle:'拖入图片或文件夹',
    emptyDesc:'自动压缩，结果保存在本地。',
    emptyDescWithUpload:'自动压缩，完成后可勾选上传到图床 / CDN。',
    queueTitle:'4 张图片待压缩',
    queueDesc:'压缩前 8.6 MB，预计输出 2.1 MB。',
    estimatedSaving:'预计节省',
    imageCountLabel:'数量',
    imageCountUnit:'张',
    compressionEffectLabel:'压缩效果',
    formatSupport:'支持 JPG · PNG · GIF · WebP · Folder',
    statusEmpty:'等待图片',
    statusQueue:'待压缩',
    statusCompressing:'压缩中',
    statusCompressed:'选择要上传的图片',
    statusUploading:'上传中',
    statusUploaded:'等待复制',
    hintEmpty:'所有操作都在当前面板完成',
    hintQueue:'准备压缩，上传进度保持为 0%',
    hintCompressed:'默认不上传，勾选后可批量上传',
    hintUploaded:'上传完成，可手动复制链接',
    actionChoose:'选择图片',
    actionCompress:'压缩',
    actionUploadSelected:'批量上传所选',
    actionNew:'新批次',
    compressLabel:'压缩',
    uploadLabel:'上传',
    selectAll:'全选',
    deselectAll:'取消全选',
    clearBatch:'清除',
    batchCleared:'当前批次已清除',
    selected:'已选',
    uploadOne:'上传此图片',
    revealLocal:'在本地定位',
    revealedLocal:'已在本地定位',
    localOnly:'仅本地',
    copyPath:'复制本地路径',
    copyLink:'复制链接',
    copyAfterUpload:'上传完成后可复制',
    waitCompress:'先压缩后上传',
    uploadedAlready:'已上传',
    copied:'已复制'
  },
  en: {
    appSubtitle:'Image compressor that trims your files',
    settingsTitle:'Settings',
    settingsDesc:'Language, compression, copy, and privacy',
    cdnTitle:'CDN settings',
    cdnDesc:'Upload target, CDN domain, and storage path',
    uploadFeatureTitle:'Upload feature',
    uploadFeatureDesc:'Show CDN upload and link actions when enabled',
    languageTitle:'Language',
    languageDesc:'Chinese by default; switch anytime',
    targetTitle:'Upload target',
    targetDesc:'Manually upload compressed files here',
    cdnDomainTitle:'CDN domain',
    cdnDomainDesc:'Public domain used when copying links',
    cdnPathTitle:'Storage path',
    cdnPathDesc:'Supports date and file-name templates',
    formatTitle:'Copy format',
    formatDesc:'Copy the CDN link after upload',
    historyTitle:'History',
    historyDesc:'Recent processed images and links',
    historyButton:'History',
    historyPrivacy:'Stores results, status, and links only. No upload secrets.',
    historyCountUnit:'records',
    historyEmpty:'No history yet',
    historyCompressed:'Compressed',
    historyUploaded:'Uploaded',
    clearHistory:'Clear',
    viewHistory:'View',
    clearHistoryDone:'History cleared',
    emptyTitle:'Drop images or folders',
    emptyDesc:'Compress locally without uploading.',
    emptyDescWithUpload:'Compress first, then select files to upload to your CDN.',
    queueTitle:'4 images ready',
    queueDesc:'8.6 MB before, estimated 2.1 MB after.',
    estimatedSaving:'estimated saving',
    imageCountLabel:'Images',
    imageCountUnit:'',
    compressionEffectLabel:'Saving',
    formatSupport:'JPG · PNG · GIF · WebP · Folder',
    statusEmpty:'Waiting for images',
    statusQueue:'Ready to compress',
    statusCompressing:'Compressing',
    statusCompressed:'Select images to upload',
    statusUploading:'Uploading',
    statusUploaded:'Ready to copy',
    hintEmpty:'Everything happens in this panel',
    hintQueue:'Ready to compress; upload remains 0%',
    hintCompressed:'Nothing uploads by default. Select images for batch upload.',
    hintUploaded:'Uploaded; copy links manually',
    actionChoose:'Choose images',
    actionCompress:'Compress',
    actionUploadSelected:'Upload selected',
    actionNew:'New batch',
    compressLabel:'Compress',
    uploadLabel:'Upload',
    selectAll:'Select all',
    deselectAll:'Deselect all',
    clearBatch:'Clear',
    batchCleared:'Current batch cleared',
    selected:'selected',
    uploadOne:'Upload this image',
    revealLocal:'Show in folder',
    revealedLocal:'Shown in folder',
    localOnly:'Local only',
    copyPath:'Copy local path',
    copyLink:'Copy link',
    copyAfterUpload:'Copy after CDN upload',
    waitCompress:'Compress before upload',
    uploadedAlready:'Uploaded',
    copied:'Copied'
  },
  ja: {
    appSubtitle:'画像を軽くする圧縮ツール',
    settingsTitle:'設定',
    settingsDesc:'言語、圧縮、コピー、プライバシー',
    cdnTitle:'CDN 設定',
    cdnDesc:'アップロード先、CDN ドメイン、保存パス',
    uploadFeatureTitle:'アップロード機能',
    uploadFeatureDesc:'有効にすると CDN 送信とリンク操作を表示',
    languageTitle:'言語',
    languageDesc:'初期設定は中国語、いつでも切替可能',
    targetTitle:'アップロード先',
    targetDesc:'圧縮後に手動でアップロード',
    cdnDomainTitle:'CDN ドメイン',
    cdnDomainDesc:'リンクコピー時に使う公開ドメイン',
    cdnPathTitle:'保存パス',
    cdnPathDesc:'日付とファイル名テンプレートに対応',
    formatTitle:'コピー形式',
    formatDesc:'アップロード後に CDN リンクをコピー',
    historyTitle:'履歴',
    historyDesc:'最近処理した画像とリンク',
    historyButton:'履歴',
    historyPrivacy:'結果、状態、リンクのみ保存します。アップロード秘密情報は保存しません。',
    historyCountUnit:'件',
    historyEmpty:'履歴はありません',
    historyCompressed:'圧縮済み',
    historyUploaded:'アップロード済み',
    clearHistory:'削除',
    viewHistory:'表示',
    clearHistoryDone:'履歴を削除しました',
    emptyTitle:'画像またはフォルダを追加',
    emptyDesc:'自動で圧縮し、結果をローカルに保存します。',
    emptyDescWithUpload:'圧縮後、選択した画像を CDN へアップロードできます。',
    queueTitle:'4 枚の画像が待機中',
    queueDesc:'圧縮前 8.6 MB、出力予測 2.1 MB。',
    estimatedSaving:'予測削減',
    imageCountLabel:'枚数',
    imageCountUnit:'枚',
    compressionEffectLabel:'圧縮効果',
    formatSupport:'JPG · PNG · GIF · WebP · Folder',
    statusEmpty:'画像待ち',
    statusQueue:'圧縮待ち',
    statusCompressing:'圧縮中',
    statusCompressed:'アップロードする画像を選択',
    statusUploading:'アップロード中',
    statusUploaded:'コピー待ち',
    hintEmpty:'すべての操作はこのパネル内で完了',
    hintQueue:'圧縮準備完了。アップロードは 0%',
    hintCompressed:'既定ではアップロードしません。選択して一括アップロードします。',
    hintUploaded:'アップロード完了。手動でコピー',
    actionChoose:'画像を選択',
    actionCompress:'圧縮',
    actionUploadSelected:'選択をアップロード',
    actionNew:'新規バッチ',
    compressLabel:'圧縮',
    uploadLabel:'アップロード',
    selectAll:'すべて選択',
    deselectAll:'選択解除',
    clearBatch:'削除',
    batchCleared:'現在のバッチを削除しました',
    selected:'選択済み',
    uploadOne:'この画像をアップロード',
    revealLocal:'ローカルで表示',
    revealedLocal:'ローカルで表示しました',
    localOnly:'ローカルのみ',
    copyPath:'ローカルパスをコピー',
    copyLink:'リンクをコピー',
    copyAfterUpload:'アップロード後にコピー可能',
    waitCompress:'先に圧縮',
    uploadedAlready:'アップロード済み',
    copied:'コピーしました'
  }
};


function mockThumb(label, colorA, colorB) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="${colorA}"/><stop offset="1" stop-color="${colorB}"/></linearGradient></defs><rect width="96" height="96" rx="18" fill="url(#g)"/><circle cx="70" cy="24" r="10" fill="white" opacity=".72"/><path d="M10 76 34 50l14 14 13-18 25 30H10Z" fill="white" opacity=".78"/><text x="14" y="24" fill="white" font-family="Arial,sans-serif" font-size="12" font-weight="700">${label}</text></svg>`;
  return 'data:image/svg+xml,' + encodeURIComponent(svg);
}

const initialFiles = [
  { name:'hero-cover.png', meta:'PNG → WebP · 2400×1600', saved:'−76%', thumb: mockThumb('PNG', '#68a6ff', '#7fd0c8') },
  { name:'product-shot.jpg', meta:'JPEG → WebP + fallback', saved:'−77%', thumb: mockThumb('JPG', '#f59e6f', '#f7cf73') },
  { name:'feature-loop.gif', meta:'GIF → animated WebP', saved:'−67%', thumb: mockThumb('GIF', '#a78bfa', '#60a5fa') },
  { name:'docs-diagram.svg', meta:'SVG minified', saved:'−85%', thumb: mockThumb('SVG', '#34d399', '#38bdf8') }
];

const initialHistory = [
  { id:'past-1', name:'landing-hero.webp', meta:'Blog CDN · R2 · −73%', statusKey:'historyUploaded', url:'https://cdn.picdog.app/2026/10/landing-hero.webp' },
  { id:'past-2', name:'release-cover.jpg', meta:'Docs OSS · −68%', statusKey:'historyUploaded', url:'https://docs-cdn.picdog.app/release-cover.jpg' },
  { id:'past-3', name:'icon-sheet.png', meta:'Local only · −51%', statusKey:'historyCompressed', url:'' }
];

const core = window.PicDogCore;
const UPLOAD_ENABLED_STORAGE_KEY = 'picdog.uploadEnabled';

if (!core) {
  throw new Error('PicDogCore is required before app.js');
}

let files = [];
let historyRecords = [...initialHistory];
let lang = 'zh';
let state = 'empty';
let timer = null;
let uploadEnabled = readStoredUploadEnabled();

const appPanel = document.querySelector('.app-panel');
const dropZone = document.querySelector('#dropZone');
const fileBoard = document.querySelector('#fileBoard');
const fileList = document.querySelector('#fileList');
const mainAction = document.querySelector('#mainAction');
const selectAllButton = document.querySelector('#selectAllButton');
const clearBatchButton = document.querySelector('#clearBatchButton');
const batchCount = document.querySelector('#batchCount');
const batchSaving = document.querySelector('#batchSaving');
const cdnButton = document.querySelector('#cdnButton');
const cdnPanel = document.querySelector('#cdnPanel');
const cdnClose = document.querySelector('#cdnClose');
const toast = document.querySelector('#toast');
const settingsButton = document.querySelector('#settingsButton');
const settingsPanel = document.querySelector('#settingsPanel');
const settingsClose = document.querySelector('#settingsClose');
const targetSelect = document.querySelector('#targetSelect');
const formatSelect = document.querySelector('#formatSelect');
const uploadEnabledSwitch = document.querySelector('#uploadEnabledSwitch');
const historyButton = document.querySelector('#historyButton');
const historyPanel = document.querySelector('#historyPanel');
const historyClose = document.querySelector('#historyClose');
const historyList = document.querySelector('#historyList');
const historyCount = document.querySelector('#historyCount');
const clearHistoryButton = document.querySelector('#clearHistoryButton');

function t(k) {
  return dictionaries[lang][k] || k;
}

function readStoredUploadEnabled() {
  try {
    return core.normalizeUploadEnabled(window.localStorage.getItem(UPLOAD_ENABLED_STORAGE_KEY));
  } catch (_error) {
    return false;
  }
}

function persistUploadEnabled(enabled) {
  try {
    window.localStorage.setItem(UPLOAD_ENABLED_STORAGE_KEY, String(enabled));
  } catch (_error) {
    // The prototype still works when storage is unavailable (for example, file:// privacy mode).
  }
}

function resetFiles() {
  files = core.cloneFiles(initialFiles);
}

function applyI18n() {
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : lang;
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    const label = t(el.dataset.i18nTitle);
    el.title = label;
    el.setAttribute('aria-label', label);
  });
  document.querySelectorAll('.lang').forEach(b => {
    b.classList.toggle('active', b.dataset.lang === lang);
  });
  updateUploadPreferenceUi();
  renderFiles();
  renderHistory();
  updateStateText();
}

function updateUploadPreferenceUi() {
  appPanel.dataset.uploadEnabled = String(uploadEnabled);
  uploadEnabledSwitch.checked = uploadEnabled;
  const emptyDescription = document.querySelector('[data-i18n="emptyDesc"]');
  if (emptyDescription) {
    emptyDescription.textContent = uploadEnabled ? t('emptyDescWithUpload') : t('emptyDesc');
  }
}

function setUploadEnabled(enabled, options = {}) {
  const nextEnabled = Boolean(enabled);
  const shouldPersist = options.persist !== false;

  if (state === 'uploading') {
    uploadEnabledSwitch.checked = uploadEnabled;
    return;
  }

  uploadEnabled = nextEnabled;
  if (!uploadEnabled) core.clearSelection(files);
  if (shouldPersist) persistUploadEnabled(uploadEnabled);

  updateUploadPreferenceUi();
  renderFiles();
  renderHistory();
  updateStateText();
}

function selectedCount() {
  return core.selectedCount(files);
}

function uploadedCount() {
  return core.uploadedCount(files);
}

function overallSaving() {
  if (!files.length) return null;

  const values = files
    .map(file => Number.parseFloat(String(file.saved).replace(/[^\d.]/g, '')))
    .filter(Number.isFinite);

  if (!values.length) return null;
  return Math.round(values.reduce((total, value) => total + value, 0) / values.length);
}

function updateBatchStats() {
  const hasBatch = state !== 'empty' && files.length > 0;
  const saving = overallSaving();
  const compressionComplete = hasBatch && files.every(file => file.compress === 100);
  const countUnit = t('imageCountUnit');

  batchCount.textContent = hasBatch
    ? files.length + (countUnit ? ' ' + countUnit : '')
    : '0' + (countUnit ? ' ' + countUnit : '');
  batchSaving.textContent = compressionComplete && saving !== null ? saving + '%' : '—';
  batchSaving.closest('.batch-stat').classList.toggle('is-positive', compressionComplete && saving !== null);
}

function updateStateText() {
  const map = {
    empty:['statusEmpty','hintEmpty','actionChoose'],
    queue:['statusQueue','hintQueue','actionCompress'],
    compressing:['statusCompressing','hintQueue','actionCompress'],
    compressed:['statusCompressed','hintCompressed','actionUploadSelected'],
    uploading:['statusUploading','hintCompressed','actionUploadSelected'],
    uploaded:['statusUploaded','hintUploaded','actionNew']
  };
  const row = map[state];
  const count = selectedCount();
  const hasPendingUploads = core.hasPendingUploads(files);

  if (!uploadEnabled && (state === 'compressed' || state === 'uploaded')) {
    mainAction.textContent = t('actionNew');
    mainAction.disabled = false;
  } else {
    mainAction.textContent = state === 'uploaded' && hasPendingUploads ? t('actionUploadSelected') : t(row[2]);
    mainAction.disabled = state === 'compressing' || state === 'uploading' || ((state === 'compressed' || (state === 'uploaded' && hasPendingUploads)) && count === 0);
  }
  selectAllButton.hidden = !uploadEnabled || !(state === 'compressed' || state === 'uploaded');
  selectAllButton.textContent = count === files.length && files.length ? t('deselectAll') : t('selectAll');
  clearBatchButton.hidden = state === 'empty' || state === 'compressing' || state === 'uploading';
  clearBatchButton.textContent = t('clearBatch');
  uploadEnabledSwitch.disabled = state === 'uploading';
  updateBatchStats();
}

function setState(next) {
  state = next;
  appPanel.dataset.state = next;
  dropZone.hidden = next !== 'empty';
  fileBoard.hidden = next === 'empty';

  if (next === 'empty') {
    resetFiles();
  }

  if (next === 'queue') {
    files.forEach(f => { f.selected = false; f.compress = 0; f.upload = 0; });
  }

  if (next === 'compressed') {
    files.forEach(f => { f.selected = false; f.compress = 100; f.upload = 0; });
    upsertHistoryFromCurrentBatch();
  }

  if (next === 'uploaded') {
    upsertHistoryFromCurrentBatch();
  }

  renderFiles();
  updateStateText();
}


function fileExtension(file) {
  return (file.name.split('.').pop() || 'img').slice(0, 3).toUpperCase();
}

function thumbMarkup(file) {
  if (file.thumb) {
    return '<span class="thumb has-preview"><img src="' + file.thumb + '" alt="" loading="lazy"></span>';
  }

  return '<span class="thumb"><span>' + fileExtension(file) + '</span></span>';
}

function renderFiles() {
  if (fileBoard.hidden) return;
  fileList.innerHTML = files.map((file, index) => {
    const canUpload = uploadEnabled && core.canUpload(state, file);
    const canCopy = uploadEnabled && core.canCopy(file);
    const copyTitle = canCopy ? t('copyLink') : t('copyAfterUpload');
    const uploadTitle = file.upload === 100 ? t('uploadedAlready') : (canUpload ? t('uploadOne') : t('waitCompress'));

    const isCompressed = file.compress === 100;
    const isUploaded = uploadEnabled && file.upload === 100;
    const compressIcon = isCompressed ? '<span class="progress-status-icon"><i class="success-icon" aria-label="success" title="success">✓</i></span>' : '<span class="progress-status-icon"></span>';
    const uploadIcon = isUploaded ? '<span class="progress-status-icon"><i class="success-icon" aria-label="success" title="success">✓</i></span>' : '<span class="progress-status-icon"></span>';
    const selectionMarkup = uploadEnabled
      ? '<label class="check-wrap" title="' + t('selected') + '"><input type="checkbox" data-check="' + index + '" ' + (file.selected ? 'checked' : '') + ' ' + (core.isRowLocked(state) ? 'disabled' : '') + '/></label>'
      : '';
    const uploadActionsMarkup = uploadEnabled
      ? '<button class="icon-action" data-upload-one="' + index + '" title="' + uploadTitle + '" ' + (!canUpload ? 'disabled' : '') + '>↗</button><button class="icon-action" data-copy-one="' + index + '" title="' + copyTitle + '" ' + (!canCopy ? 'disabled' : '') + '>⧉</button>'
      : '';
    const revealMarkup = '<button class="icon-action" data-reveal-one="' + index + '" title="' + t('revealLocal') + '">⌕</button>';
    const uploadProgressMarkup = uploadEnabled
      ? '<span class="progress-label upload-label"><b>' + t('uploadLabel') + '</b></span><div class="track upload-track"><div class="bar upload' + (isUploaded ? ' is-success' : '') + '" style="--value:' + file.upload + '%"></div></div>' + uploadIcon + '<span class="progress-percent upload-percent">' + file.upload + '%</span>'
      : '';

    return '<article class="file-card' + (isUploaded ? ' is-uploaded' : '') + '">'
      + selectionMarkup
      + thumbMarkup(file)
      + '<div class="file-main">'
      + '<div class="file-top"><div><strong>' + file.name + '</strong><small>' + file.meta + ' · ' + file.saved + '</small></div>'
      + '<div class="icon-actions">' + revealMarkup + uploadActionsMarkup + '</div></div>'
      + '<div class="progress-line' + (isCompressed ? ' is-compressed' : '') + (isUploaded ? ' is-success' : '') + '"><span class="progress-label compress-label"><b>' + t('compressLabel') + '</b></span><div class="track compress-track"><div class="bar compress' + (isCompressed ? ' is-success' : '') + '" style="--value:' + file.compress + '%"></div></div>' + compressIcon + '<span class="progress-percent compress-percent">' + file.compress + '%</span>' + uploadProgressMarkup + '</div>'
      + '</div></article>';
  }).join('');
  bindRowEvents();
}

function bindRowEvents() {
  document.querySelectorAll('[data-check]').forEach(el => {
    el.addEventListener('change', e => {
      files[Number(e.target.dataset.check)].selected = e.target.checked;
      updateStateText();
    });
  });
  document.querySelectorAll('[data-upload-one]').forEach(btn => {
    btn.addEventListener('click', () => uploadIndexes([Number(btn.dataset.uploadOne)]));
  });
  document.querySelectorAll('[data-copy-one]').forEach(btn => {
    btn.addEventListener('click', () => {
      if (!btn.disabled) showToast(t('copied'));
    });
  });
  document.querySelectorAll('[data-reveal-one]').forEach(btn => {
    btn.addEventListener('click', () => showToast(t('revealedLocal')));
  });
}

function renderHistory() {
  const empty = historyRecords.length === 0;
  if (clearHistoryButton) clearHistoryButton.disabled = empty;
  if (historyButton) historyButton.dataset.count = String(historyRecords.length);
  if (historyCount) {
    historyCount.textContent = empty ? t('historyEmpty') : historyRecords.length + ' ' + t('historyCountUnit');
  }

  if (!historyList) return;

  if (empty) {
    historyList.innerHTML = '<div class="history-empty">' + t('historyEmpty') + '</div>';
    return;
  }

  historyList.innerHTML = historyRecords.slice(0, 5).map(record => {
    const canCopy = Boolean(record.url);
    const copyTitle = canCopy ? t('copyLink') : t('copyAfterUpload');
    const copyMarkup = uploadEnabled
      ? '<button class="icon-action tiny" data-copy-history="' + record.id + '" title="' + copyTitle + '" ' + (!canCopy ? 'disabled' : '') + '>⧉</button>'
      : '';

    return '<article class="history-item' + (uploadEnabled ? '' : ' local-only') + '">'
      + '<span class="history-thumb"></span>'
      + '<div class="history-main"><strong>' + record.name + '</strong><small>' + record.meta + '</small></div>'
      + '<span class="history-state ' + (canCopy ? 'uploaded' : '') + '">' + t(record.statusKey) + '</span>'
      + copyMarkup
      + '</article>';
  }).join('');

  document.querySelectorAll('[data-copy-history]').forEach(btn => {
    btn.addEventListener('click', () => {
      const record = historyRecords.find(item => item.id === btn.dataset.copyHistory);
      if (record && record.url) showToast(t('copied'));
    });
  });
}

function upsertHistoryFromCurrentBatch() {
  historyRecords = core.upsertHistoryFromFiles(historyRecords, files, uploadEnabled ? targetSelect.value : t('localOnly'), 8);
  renderHistory();
}

function animateCompress() {
  state = 'compressing';
  updateStateText();
  let value = 0;
  clearInterval(timer);
  timer = setInterval(() => {
    value = Math.min(100, value + 16);
    core.setAllCompression(files, value);
    renderFiles();
    if (value === 100) {
      clearInterval(timer);
      setTimeout(() => setState('compressed'), 240);
    }
  }, 170);
}

function uploadIndexes(indexes) {
  if (!uploadEnabled) return;
  if (state !== 'compressed' && state !== 'uploaded') return;
  const validIndexes = indexes.filter(i => files[i] && files[i].upload < 100);
  if (!validIndexes.length) return;

  state = 'uploading';
  updateStateText();
  let value = 0;
  clearInterval(timer);
  timer = setInterval(() => {
    value = Math.min(100, value + 20);
    core.setUploadProgress(files, validIndexes, value);
    renderFiles();

    if (value === 100) {
      clearInterval(timer);
      core.clearSelection(files);
      state = core.nextStateAfterUpload(files);
      upsertHistoryFromCurrentBatch();
      renderFiles();
      updateStateText();
    }
  }, 180);
}

function handleMain() {
  if (state === 'empty') return setState('queue');
  if (state === 'queue') return animateCompress();
  if (!uploadEnabled && (state === 'compressed' || state === 'uploaded')) return setState('empty');
  if (state === 'compressed' || (state === 'uploaded' && uploadedCount() < files.length)) {
    const idx = core.selectedUploadIndexes(files);
    return uploadIndexes(idx);
  }
  if (state === 'uploaded') return setState('empty');
}

function showToast(msg) {
  toast.textContent = msg;
  toast.classList.add('show');
  setTimeout(() => toast.classList.remove('show'), 1400);
}

function setSettingsOpen(open) {
  settingsPanel.hidden = !open;
  settingsButton.setAttribute('aria-expanded', String(open));
  if (open) {
    setCdnOpen(false);
    setHistoryOpen(false);
  }
}

function setCdnOpen(open) {
  cdnPanel.hidden = !open;
  cdnButton.setAttribute('aria-expanded', String(open));
  if (open) {
    setSettingsOpen(false);
    setHistoryOpen(false);
  }
}

function setHistoryOpen(open) {
  historyPanel.hidden = !open;
  historyButton.setAttribute('aria-expanded', String(open));
  if (open) {
    setCdnOpen(false);
    setSettingsOpen(false);
    renderHistory();
  }
}

mainAction.addEventListener('click', handleMain);
selectAllButton.addEventListener('click', () => {
  if (!uploadEnabled) return;
  core.toggleSelectableFiles(files);
  renderFiles();
  updateStateText();
});

clearBatchButton.addEventListener('click', () => {
  if (state === 'compressing' || state === 'uploading') return;
  setState('empty');
  showToast(t('batchCleared'));
});

function clearHistoryRecords() {
  historyRecords = [];
  renderHistory();
  showToast(t('clearHistoryDone'));
}

clearHistoryButton.addEventListener('click', clearHistoryRecords);
historyButton.addEventListener('click', e => {
  e.stopPropagation();
  setHistoryOpen(historyPanel.hidden);
});
historyClose.addEventListener('click', () => setHistoryOpen(false));
historyPanel.addEventListener('click', e => e.stopPropagation());

formatSelect.addEventListener('change', () => showToast(formatSelect.value));
targetSelect.addEventListener('change', () => showToast(targetSelect.value));
uploadEnabledSwitch.addEventListener('change', event => {
  setUploadEnabled(event.target.checked);
});
document.querySelectorAll('.lang').forEach(b => {
  b.addEventListener('click', () => {
    lang = b.dataset.lang;
    applyI18n();
  });
});

cdnButton.addEventListener('click', e => {
  e.stopPropagation();
  setCdnOpen(cdnPanel.hidden);
});
cdnClose.addEventListener('click', () => setCdnOpen(false));
cdnPanel.addEventListener('click', e => e.stopPropagation());

settingsButton.addEventListener('click', e => {
  e.stopPropagation();
  setSettingsOpen(settingsPanel.hidden);
});
settingsClose.addEventListener('click', () => setSettingsOpen(false));
settingsPanel.addEventListener('click', e => e.stopPropagation());
document.addEventListener('click', () => { setSettingsOpen(false); setCdnOpen(false); setHistoryOpen(false); });
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') { setSettingsOpen(false); setCdnOpen(false); setHistoryOpen(false); }
});

resetFiles();
applyI18n();
setState('empty');
