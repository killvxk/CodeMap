import { describe, it, expect, afterAll } from 'vitest';
import { execFileSync } from 'child_process';
import fs from 'fs/promises';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, '../fixtures/sample-project');
const CODEMAP_DIR = path.join(FIXTURE_DIR, '.codemap');
const BIN = path.resolve(import.meta.dirname, '../../bin/codegraph.js');

describe('scan command', () => {
  afterAll(async () => {
    // Clean up .codemap/ in fixture dir after tests
    await fs.rm(CODEMAP_DIR, { recursive: true, force: true });
  });

  it('should create graph.json, meta.json, and slices/_overview.json', () => {
    execFileSync('node', [BIN, 'scan', FIXTURE_DIR], {
      encoding: 'utf-8',
      timeout: 30000,
    });

    // Verify files exist by attempting to read them
    const graphExists = fs.access(path.join(CODEMAP_DIR, 'graph.json'));
    const metaExists = fs.access(path.join(CODEMAP_DIR, 'meta.json'));
    const overviewExists = fs.access(path.join(CODEMAP_DIR, 'slices', '_overview.json'));

    return Promise.all([graphExists, metaExists, overviewExists]);
  });

  it('graph.json should be valid JSON with version "1.0"', async () => {
    const content = await fs.readFile(path.join(CODEMAP_DIR, 'graph.json'), 'utf-8');
    const graph = JSON.parse(content);
    expect(graph.version).toBe('1.0');
    expect(graph.summary.totalFiles).toBeGreaterThan(0);
  });

  it('meta.json should contain lastScanAt and fileHashes', async () => {
    const content = await fs.readFile(path.join(CODEMAP_DIR, 'meta.json'), 'utf-8');
    const meta = JSON.parse(content);
    expect(meta.lastScanAt).toBeDefined();
    expect(meta.fileHashes).toBeDefined();
    expect(typeof meta.scanDuration).toBe('number');
  });

  it('slices/_overview.json should be valid JSON with modules', async () => {
    const content = await fs.readFile(path.join(CODEMAP_DIR, 'slices', '_overview.json'), 'utf-8');
    const overview = JSON.parse(content);
    expect(overview.project).toBeDefined();
    expect(overview.modules).toBeInstanceOf(Array);
    expect(overview.modules.length).toBeGreaterThan(0);
  });

  it('should produce per-module slice files', async () => {
    const slicesDir = path.join(CODEMAP_DIR, 'slices');
    const files = await fs.readdir(slicesDir);
    expect(files).toContain('auth.json');
    expect(files).toContain('api.json');
  });
});
