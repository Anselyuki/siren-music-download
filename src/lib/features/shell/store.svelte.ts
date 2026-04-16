let settingsOpen = $state(false);
let downloadPanelOpen = $state(false);
let initialized = false;

function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  initialized = false;
}

export const shellStore = {
  get settingsOpen() {
    return settingsOpen;
  },
  get downloadPanelOpen() {
    return downloadPanelOpen;
  },
  openSettings() {
    settingsOpen = true;
  },
  closeSettings() {
    settingsOpen = false;
  },
  openDownloads() {
    downloadPanelOpen = true;
  },
  closeDownloads() {
    downloadPanelOpen = false;
  },
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
