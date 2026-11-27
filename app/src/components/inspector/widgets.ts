// The live widgets the service draws, for the gallery. `lhm`: on Windows the temperature
// sensors need LibreHardwareMonitor running.

import { Bot, Clock, Cpu, Gpu, MemoryStick, Music2, Thermometer, Volume2 } from 'lucide-svelte';
import type { Widget } from '../../lib/types';
import type { Icon } from '../ui/icon';

export const WIDGETS: { kind: Widget['kind']; label: string; icon: Icon; lhm?: boolean }[] = [
  { kind: 'clock', label: 'Clock', icon: Clock },
  { kind: 'cpu_usage', label: 'CPU usage', icon: Cpu },
  { kind: 'cpu_temp', label: 'CPU temp', icon: Thermometer, lhm: true },
  { kind: 'ram_usage', label: 'RAM', icon: MemoryStick },
  { kind: 'gpu_usage', label: 'GPU usage', icon: Gpu },
  { kind: 'gpu_temp', label: 'GPU temp', icon: Thermometer, lhm: true },
  { kind: 'volume', label: 'Volume', icon: Volume2 },
  { kind: 'now_playing', label: 'Now playing', icon: Music2 },
  { kind: 'claude_session', label: 'Claude session', icon: Bot },
];
