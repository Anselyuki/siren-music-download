export interface Album {
  cid: string;
  name: string;
  coverUrl: string;
  artists: string[];
}

export interface SongEntry {
  cid: string;
  name: string;
  artists: string[];
}

export interface PlaybackQueueEntry {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

export interface PlaybackContext {
  entries: PlaybackQueueEntry[];
  currentIndex: number;
}

export interface SongDetail {
  cid: string;
  name: string;
  albumCid: string;
  sourceUrl: string;
  lyricUrl: string | null;
  mvUrl: string | null;
  mvCoverUrl: string | null;
  artists: string[];
}

export interface AlbumDetail {
  cid: string;
  name: string;
  intro: string | null;
  belong: string;
  coverUrl: string;
  coverDeUrl: string | null;
  artists: string[] | null;
  songs: SongEntry[];
}

export interface ThemePalette {
  accentHex: string;
  accentHoverHex: string;
  accentRgb: [number, number, number];
  accentHoverRgb: [number, number, number];
}

export type OutputFormat = 'flac' | 'wav' | 'mp3';

export interface PlayerState {
  songCid: string | null;
  songName: string | null;
  artists: string[];
  coverUrl: string | null;
  isPlaying: boolean;
  isPaused: boolean;
  isLoading: boolean;
  hasPrevious: boolean;
  hasNext: boolean;
  progress: number;
  duration: number;
  volume: number;
}
