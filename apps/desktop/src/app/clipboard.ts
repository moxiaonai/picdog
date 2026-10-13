import { invoke } from '@tauri-apps/api/core';

function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function writeClipboardText(text: string) {
  if (hasTauriRuntime()) {
    await invoke('write_clipboard_text', { text });
    return;
  }

  if (!navigator.clipboard?.writeText) {
    throw new Error('系统剪贴板不可用，请检查应用权限或手动复制。');
  }

  await navigator.clipboard.writeText(text);
}
