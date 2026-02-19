import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../../src/parser.js';
import path from 'path';

const FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project/src/native/engine.cpp');

describe('C++ adapter', () => {
  beforeAll(async () => { await initParser(); });

  it('should extract functions', async () => {
    const result = await parseFile(FIXTURE, 'cpp');
    const names = result.functions.map(f => f.name);
    expect(names.some(n => n.includes('start') || n.includes('Engine'))).toBe(true);
    expect(names.some(n => n.includes('initialize'))).toBe(true);
  });

  it('should extract #include imports', async () => {
    const result = await parseFile(FIXTURE, 'cpp');
    expect(result.imports.length).toBeGreaterThanOrEqual(3);
    const iostream = result.imports.find(i => i.source.includes('iostream'));
    expect(iostream).toBeDefined();
    expect(iostream.isExternal).toBe(true);
    const utils = result.imports.find(i => i.source.includes('utils.h'));
    expect(utils).toBeDefined();
    expect(utils.isExternal).toBe(false);
  });

  it('should detect non-static exports', async () => {
    const result = await parseFile(FIXTURE, 'cpp');
    expect(result.exports).toContain('initialize');
    // static functions should not be exported
    expect(result.exports).not.toContain('internalHelper');
  });

  it('should extract classes and structs', async () => {
    const result = await parseFile(FIXTURE, 'cpp');
    const names = result.classes.map(c => c.name);
    expect(names).toContain('Engine');
    expect(names).toContain('Config');
  });

  it('should extract types', async () => {
    const result = await parseFile(FIXTURE, 'cpp');
    expect(result.types.some(t => t.name === 'Engine' && t.kind === 'class')).toBe(true);
    expect(result.types.some(t => t.name === 'Config' && t.kind === 'struct')).toBe(true);
    expect(result.types.some(t => t.name === 'ErrorCode' && t.kind === 'enum')).toBe(true);
  });
});
