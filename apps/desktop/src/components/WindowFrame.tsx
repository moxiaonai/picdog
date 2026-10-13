import { useEffect, useState, type ReactNode } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

type WindowFrameProps = {
  cdnLabel: string;
  settingsLabel: string;
  historyLabel: string;
  onOpenCdnSettings: () => void;
  onOpenSettings: () => void;
  onOpenHistory: () => void;
  children: ReactNode;
};

function hasTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function minimizeWindow() {
  if (!hasTauriRuntime()) return;
  await getCurrentWindow().minimize();
}

async function closeWindow() {
  if (!hasTauriRuntime()) return;
  await getCurrentWindow().close();
}

export function WindowFrame({
  cdnLabel,
  settingsLabel,
  historyLabel,
  onOpenCdnSettings,
  onOpenSettings,
  onOpenHistory,
  children
}: WindowFrameProps) {
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const syncFullscreenState = async () => {
      const nextFullscreen = await appWindow.isFullscreen();
      if (!disposed) setFullscreen(nextFullscreen);
    };

    void syncFullscreenState();
    void appWindow.onResized(() => void syncFullscreenState()).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const toggleFullscreen = async () => {
    if (!hasTauriRuntime()) return;

    const appWindow = getCurrentWindow();
    const nextFullscreen = !(await appWindow.isFullscreen());
    await appWindow.setFullscreen(nextFullscreen);
    setFullscreen(nextFullscreen);
  };

  return (
    <main className="scene" aria-label="PicDog desktop app">
      <section className="app-panel">
        <header className="titlebar" data-tauri-drag-region>
          <div className="traffic">
            <button type="button" aria-label="Close" onClick={() => void closeWindow()} />
            <button type="button" aria-label="Minimize" onClick={() => void minimizeWindow()} />
            <button
              type="button"
              aria-label={fullscreen ? 'Exit Full Screen' : 'Enter Full Screen'}
              aria-pressed={fullscreen}
              title={fullscreen ? 'Exit Full Screen' : 'Enter Full Screen'}
              onClick={() => void toggleFullscreen()}
            />
          </div>
          <strong className="app-title" data-tauri-drag-region>PicDog</strong>
          <div className="title-actions">
            <button className="cdn-trigger" type="button" onClick={onOpenCdnSettings} aria-label={cdnLabel} title={cdnLabel}>
              CDN
            </button>
            <button className="icon-trigger" type="button" aria-label={historyLabel} onClick={onOpenHistory}>
              ◴
            </button>
            <button className="icon-trigger" type="button" aria-label={settingsLabel} onClick={onOpenSettings}>
              ⚙︎
            </button>
          </div>
        </header>
        {children}
      </section>
    </main>
  );
}
