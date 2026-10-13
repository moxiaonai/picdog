import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app/App';
import { CompareWindow } from './features/batch/CompareWindow';
import './app/theme.css';

const isTauriRuntime =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

document.documentElement.classList.toggle('tauri-runtime', isTauriRuntime);
document.documentElement.classList.toggle('browser-preview', !isTauriRuntime);

const view = new URLSearchParams(window.location.search).get('view');
const Root = view === 'compare' ? CompareWindow : App;

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>
);
