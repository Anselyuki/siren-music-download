import type { Locale } from './types';

export function formatByteSize(bytes: number, locale: Locale): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex++;
  }
  const formatted = new Intl.NumberFormat(locale, {
    maximumFractionDigits: unitIndex === 0 ? 0 : 2,
  }).format(value);
  return `${formatted} ${units[unitIndex]}`;
}

export function formatSpeed(bytesPerSec: number, locale: Locale): string {
  return `${formatByteSize(bytesPerSec, locale)}/s`;
}

export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}
