import { mount } from 'svelte';
import './styles/global.css';
import App from './App.svelte';
import { installGlobalErrorHandlers } from './lib/globalErrors';
import { toasts } from './lib/state/dialogs.svelte';

installGlobalErrorHandlers(window, (message) => toasts.error(message));

const app = mount(App, { target: document.getElementById('app')! });

export default app;
