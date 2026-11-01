import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import './ui/tokens.css';
import { establishSession } from './session';
establishSession(window.location, window.history).then(() => {
  createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>);
}).catch(error => {
  document.getElementById('root')!.textContent = String(error.message);
});
