import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app/App';
import './app/theme.css';

const isTauriRuntime =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

document.documentElement.classList.toggle('tauri-runtime', isTauriRuntime);
document.documentElement.classList.toggle('browser-preview', !isTauriRuntime);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
