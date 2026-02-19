import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { generateOverview, generateSlices, getModuleSliceWithDeps, saveSlices } from '../src/slicer.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import fs from 'fs/promises';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('slicer', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  describe('generateOverview', () => {
    it('should return a compact overview with summary and modules', () => {
      const overview = generateOverview(graph);
      expect(overview.project).toBeDefined();
      expect(overview.scannedAt).toBeDefined();
      expect(overview.summary).toBeDefined();
      expect(overview.modules).toBeInstanceOf(Array);
      expect(overview.modules.length).toBeGreaterThan(0);
    });

    it('should be compact (JSON < 5000 chars for small fixture)', () => {
      const overview = generateOverview(graph);
      const json = JSON.stringify(overview);
      expect(json.length).toBeLessThan(5000);
    });

    it('should include simplified module info with fileCount and export names', () => {
      const overview = generateOverview(graph);
      const authMod = overview.modules.find(m => m.name === 'auth');
      expect(authMod).toBeDefined();
      expect(authMod.fileCount).toBeGreaterThan(0);
      expect(authMod.exports).toBeInstanceOf(Array);
      expect(authMod.dependsOn).toBeInstanceOf(Array);
      expect(authMod.dependedBy).toBeInstanceOf(Array);
    });

    it('should list entry points', () => {
      const overview = generateOverview(graph);
      expect(overview.entryPoints).toBeInstanceOf(Array);
    });
  });

  describe('generateSlices', () => {
    it('should return an object keyed by module name', () => {
      const slices = generateSlices(graph);
      expect(slices).toHaveProperty('auth');
      expect(slices).toHaveProperty('api');
    });

    it('should include file details in module slices', () => {
      const slices = generateSlices(graph);
      const authSlice = slices['auth'];
      expect(authSlice.files).toBeInstanceOf(Array);
      expect(authSlice.files.length).toBeGreaterThan(0);
      expect(authSlice.files[0]).toHaveProperty('functions');
      expect(authSlice.files[0]).toHaveProperty('imports');
    });

    it('should include dependency info (api depends on auth)', () => {
      const slices = generateSlices(graph);
      expect(slices['api'].dependsOn).toContain('auth');
    });

    it('should include module-level stats', () => {
      const slices = generateSlices(graph);
      const authSlice = slices['auth'];
      expect(authSlice.stats).toBeDefined();
      expect(authSlice.stats.totalFiles).toBeGreaterThan(0);
      expect(authSlice.stats.totalFunctions).toBeGreaterThanOrEqual(0);
    });
  });

  describe('getModuleSliceWithDeps', () => {
    it('should return the target module full slice', () => {
      const result = getModuleSliceWithDeps(graph, 'api');
      expect(result.module).toBe('api');
      expect(result.files).toBeInstanceOf(Array);
    });

    it('should include simplified dependency info', () => {
      const result = getModuleSliceWithDeps(graph, 'api');
      expect(result.dependencies).toBeInstanceOf(Array);
      const authDep = result.dependencies.find(d => d.name === 'auth');
      expect(authDep).toBeDefined();
      expect(authDep.exports).toBeInstanceOf(Array);
    });
  });

  describe('saveSlices', () => {
    const outputDir = path.resolve(FIXTURE_DIR, '.codemap-test-slices');

    afterAll(async () => {
      await fs.rm(outputDir, { recursive: true, force: true });
    });

    it('should write _overview.json and per-module files', async () => {
      await saveSlices(outputDir, graph);
      const slicesDir = path.join(outputDir, 'slices');
      const files = await fs.readdir(slicesDir);
      expect(files).toContain('_overview.json');
      expect(files).toContain('auth.json');
      expect(files).toContain('api.json');

      // Verify _overview.json is valid JSON
      const overviewContent = await fs.readFile(path.join(slicesDir, '_overview.json'), 'utf-8');
      const overview = JSON.parse(overviewContent);
      expect(overview.project).toBeDefined();
    });
  });
});
