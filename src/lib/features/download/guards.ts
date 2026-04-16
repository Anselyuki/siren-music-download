import type { DownloadJobSnapshot } from "$lib/types";

export function hasCurrentDownloadOptions(
  job: DownloadJobSnapshot,
  outputDir: string,
  format: string,
  downloadLyrics: boolean,
): boolean {
  return (
    job.options.outputDir === outputDir &&
    job.options.format === format &&
    job.options.downloadLyrics === downloadLyrics
  );
}

export function isJobActive(job: DownloadJobSnapshot): boolean {
  return job.status === "queued" || job.status === "running";
}
