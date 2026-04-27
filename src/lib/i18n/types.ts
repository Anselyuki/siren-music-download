export type Locale = 'zh-CN' | 'en-US';

export const SUPPORTED_LOCALES: readonly Locale[] = ['zh-CN', 'en-US'] as const;

export const BOOTSTRAP_LOCALE_FALLBACK: Locale = 'zh-CN';
