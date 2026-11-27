// Entry of the switch overlay window (overlay.html): the toast near the tray. It shares the
// theme engine with the app, so it looks like whichever theme is chosen.

import { mount } from 'svelte';
import '../app.css';
import { applyCached, initTheme, loadUserThemes } from '../lib/theme/theme.svelte';
import Overlay from './Overlay.svelte';

applyCached();
initTheme();
void loadUserThemes();

export default mount(Overlay, { target: document.getElementById('app')! });
