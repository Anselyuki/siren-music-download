import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(__dirname, '../../../..');
const appCss = readFileSync(resolve(repoRoot, 'src/app.css'), 'utf8');
const albumSidebar = readFileSync(
  resolve(repoRoot, 'src/lib/components/app/AlbumSidebar.svelte'),
  'utf8'
);

function getRule(source: string, selector: string): string {
  const selectorIndex = source.indexOf(selector);
  expect(selectorIndex, `${selector} should exist`).toBeGreaterThanOrEqual(0);

  const blockStart = source.indexOf('{', selectorIndex);
  expect(blockStart, `${selector} should have a CSS block`).toBeGreaterThan(0);

  let depth = 0;
  for (let index = blockStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) {
      return source.slice(blockStart + 1, index);
    }
  }

  throw new Error(`Unclosed CSS block for ${selector}`);
}

function expectDeclaration(rule: string, property: string, value: string) {
  expect(rule).toContain(`${property}: ${value};`);
}

describe('AlbumSidebar layout width contract', () => {
  it('keeps the app sidebar column compressed', () => {
    const containerRule = getRule(appCss, '.container');
    const sidebarRule = getRule(appCss, '.sidebar');
    const sidebarLayoutRule = getRule(appCss, '.sidebar .sidebar-layout');
    const macosSidebarLayoutRule = getRule(
      appCss,
      '.container.macos-overlay .sidebar .sidebar-layout'
    );

    expectDeclaration(containerRule, 'grid-template-columns', '248px 1fr');
    expectDeclaration(sidebarRule, 'width', '248px');
    expectDeclaration(sidebarRule, 'padding', '0');
    expectDeclaration(sidebarRule, 'min-width', '0');
    expectDeclaration(sidebarLayoutRule, 'padding', '24px 16px');
    expectDeclaration(macosSidebarLayoutRule, 'padding-top', '60px');
  });

  it('prevents the sidebar contents from reserving extra right-side width', () => {
    const layoutRule = getRule(albumSidebar, '.sidebar-layout');
    const scrollAreaRule = getRule(albumSidebar, '.sidebar-scroll-area');
    const albumListRule = getRule(
      albumSidebar,
      '.sidebar-layout :global(.album-list)'
    );
    const albumCardRule = getRule(
      albumSidebar,
      '.sidebar-layout :global(.album-card)'
    );

    expectDeclaration(layoutRule, 'width', '100%');
    expectDeclaration(layoutRule, 'min-width', '0');
    expectDeclaration(scrollAreaRule, 'width', '100%');
    expectDeclaration(scrollAreaRule, 'padding-right', '0');
    expectDeclaration(albumListRule, 'width', '100%');
    expectDeclaration(albumListRule, 'min-width', '0');
    expectDeclaration(albumCardRule, 'width', '100%');
    expectDeclaration(albumCardRule, 'min-width', '0');
  });
});
