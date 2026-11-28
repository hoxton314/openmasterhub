import { mount } from 'svelte';
import './app.css';
import { applyCached, initTheme } from './lib/theme/theme.svelte';

// Paint the last theme before mounting (no flash), then resolve the chosen one.
applyCached();
initTheme();

const target = document.getElementById('app')!;
// `?gallery` (dev builds): every ui/ component in every theme.
const gallery = import.meta.env.DEV && new URLSearchParams(location.search).has('gallery');
const app = gallery ? import('./components/shell/Gallery.svelte') : import('./App.svelte');
export default app.then((m) => mount(m.default, { target }));
