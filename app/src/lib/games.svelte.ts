// The Game modes section: which games `masterhub-games` offers and which one is being set up.

import { invoke } from './backend';
import type { GameStatus } from './types';

export const games = $state({
  /** Empty when masterhub-games is not installed. */
  list: [] as GameStatus[],
  /** The game whose setup dialog is open. */
  open: null as GameStatus | null,
});

export async function refreshGames() {
  games.list = await invoke<GameStatus[]>('games_list').catch(() => []);
}
