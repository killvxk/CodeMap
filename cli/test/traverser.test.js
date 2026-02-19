import { describe, it, expect } from 'vitest';
import { traverseFiles } from '../src/traverser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('traverseFiles', () => {
  it('should find source files excluding node_modules', async () => {
    const files = await traverseFiles(FIXTURE_DIR);
    const relative = files.map(f => path.relative(FIXTURE_DIR, f).replace(/\\/g, '/'));
    expect(relative).toContain('src/auth/login.ts');
    expect(relative).toContain('src/api/routes.ts');
    expect(relative.some(f => f.includes('node_modules'))).toBe(false);
  });

  it('should filter by language extensions', async () => {
    const files = await traverseFiles(FIXTURE_DIR, { extensions: ['.ts'] });
    expect(files.every(f => f.endsWith('.ts'))).toBe(true);
  });

  it('should respect custom exclude patterns', async () => {
    const files = await traverseFiles(FIXTURE_DIR, { exclude: ['**/api/**'] });
    const relative = files.map(f => path.relative(FIXTURE_DIR, f).replace(/\\/g, '/'));
    expect(relative.some(f => f.includes('api'))).toBe(false);
  });
});
