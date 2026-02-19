import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';
import os from 'os';

const SOURCE_FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project');
const CLI_BIN = path.resolve(import.meta.dirname, '../../bin/codegraph.js');

// Use a temporary copy of the fixture to avoid conflicts with other test files
// that share the same fixture directory (e.g. commands/scan.test.js).
let WORK_DIR;
let CODEMAP_DIR;

function run(cmd) {
  return execSync(`node "${CLI_BIN}" ${cmd}`, {
    encoding: 'utf-8',
    cwd: WORK_DIR,
    timeout: 30000,
  });
}

/**
 * Recursively copy a directory (synchronous).
 */
function copyDirSync(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === '.codemap') continue;
      copyDirSync(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

describe('E2E: full workflow', () => {
  beforeAll(() => {
    // Create isolated temp copy of the sample-project fixture
    WORK_DIR = path.join(os.tmpdir(), `codemap-e2e-${Date.now()}`);
    copyDirSync(SOURCE_FIXTURE, WORK_DIR);
    CODEMAP_DIR = path.join(WORK_DIR, '.codemap');
  });

  afterAll(() => {
    fs.rmSync(WORK_DIR, { recursive: true, force: true });
  });

  it('scan generates .codemap with all expected files', () => {
    const output = run('scan .');
    expect(output).toContain('Scan complete');
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'graph.json'))).toBe(true);
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'meta.json'))).toBe(true);
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'slices', '_overview.json'))).toBe(true);
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'slices', 'auth.json'))).toBe(true);
  });

  it('status shows correct project info', () => {
    const output = run('status');
    expect(output).toContain('Files:');
  });

  it('query finds the login function', () => {
    const output = run('query login');
    expect(output.toLowerCase()).toContain('login');
  });

  it('slice outputs valid overview JSON', () => {
    const output = run('slice');
    const overview = JSON.parse(output);
    expect(overview.summary || overview.project).toBeDefined();
  });

  it('slice with module outputs module data', () => {
    const output = run('slice auth --with-deps');
    const data = JSON.parse(output);
    expect(data.target || data.module).toBeDefined();
  });

  it('impact analysis returns results for auth', () => {
    const output = run('impact auth');
    expect(output.toLowerCase()).toContain('auth');
  });

  it('update detects no changes after fresh scan', () => {
    const output = run('update');
    expect(output).toContain('No changes');
  });
});
