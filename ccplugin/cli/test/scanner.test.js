import { describe, it, expect, beforeAll } from 'vitest';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('scanProject', () => {
  beforeAll(async () => {
    await initParser();
  });

  it('should produce a valid graph with summary', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    expect(graph.version).toBe('1.0');
    expect(graph.summary.totalFiles).toBeGreaterThan(0);
    expect(graph.summary.languages).toHaveProperty('typescript');
  });

  it('should detect modules from directory structure', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    expect(graph.summary.modules).toContain('auth');
    expect(graph.summary.modules).toContain('api');
  });

  it('should extract file-level details', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    const loginFile = Object.keys(graph.files).find(f => f.includes('login.ts'));
    expect(loginFile).toBeDefined();
    const fileData = graph.files[loginFile];
    expect(fileData.functions.length).toBeGreaterThan(0);
    expect(fileData.imports.length).toBeGreaterThan(0);
    expect(fileData.exports).toContain('login');
  });

  it('should build module dependency graph', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    expect(graph.modules['api']).toBeDefined();
    expect(graph.modules['api'].dependsOn).toContain('auth');
  });
});
