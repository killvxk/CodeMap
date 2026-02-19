import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../../src/parser.js';
import path from 'path';

const FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project/src/utils/helpers.py');

describe('Python adapter', () => {
  beforeAll(async () => { await initParser(); });

  it('should extract functions', async () => {
    const result = await parseFile(FIXTURE, 'python');
    const names = result.functions.map(f => f.name);
    expect(names).toContain('process_data');
    expect(names).toContain('_internal_helper');
  });

  it('should extract function signatures with type hints', async () => {
    const result = await parseFile(FIXTURE, 'python');
    const fn = result.functions.find(f => f.name === 'process_data');
    expect(fn.signature).toContain('input_path');
  });

  it('should extract imports', async () => {
    const result = await parseFile(FIXTURE, 'python');
    expect(result.imports.length).toBeGreaterThanOrEqual(3);
    const osImport = result.imports.find(i => i.source === 'os');
    expect(osImport).toBeDefined();
    expect(osImport.isExternal).toBe(true);
    const pathImport = result.imports.find(i => i.source === 'os.path');
    expect(pathImport).toBeDefined();
    expect(pathImport.symbols).toContain('join');
  });

  it('should extract exports from __all__', async () => {
    const result = await parseFile(FIXTURE, 'python');
    expect(result.exports).toContain('process_data');
    expect(result.exports).toContain('DataProcessor');
  });

  it('should extract classes', async () => {
    const result = await parseFile(FIXTURE, 'python');
    expect(result.classes.length).toBe(1);
    expect(result.classes[0].name).toBe('DataProcessor');
  });
});
