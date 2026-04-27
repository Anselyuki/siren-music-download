import { setLocale as setParaglideLocale } from '$lib/paraglide/runtime.js';
import { BOOTSTRAP_LOCALE_FALLBACK, type Locale } from './types';

class LocaleState {
  current = $state<Locale>(BOOTSTRAP_LOCALE_FALLBACK);

  applyBackendLocale(locale: Locale) {
    if (this.current === locale) return;
    this.current = locale;
    void setParaglideLocale(locale, { reload: false });
    document.documentElement.lang = locale;
  }
}

export const localeState = new LocaleState();
