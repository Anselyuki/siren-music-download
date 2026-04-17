import type { DownloadTaskSnapshot } from "$lib/types";

export function buildSelectionKey(songCids: string[]): string {
  return [...songCids].sort().join(",");
}

export function formatByteSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes < 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function formatSpeed(bytesPerSec: number): string {
  return `${formatByteSize(bytesPerSec)}/s`;
}

export function getTaskStatusLabel(task: DownloadTaskSnapshot): string {
  return task.status;
}
