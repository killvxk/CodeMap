import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../../src/parser.js';
import path from 'path';

const FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project/src/core/engine.rs');

describe('Rust adapter', () => {
  beforeAll(async () => { await initParser(); });

  it('should extract functions including impl methods', async () => {
    const result = await parseFile(FIXTURE, 'rust');
    const names = result.functions.map(f => f.name);
    expect(names).toContain('public_function');
    expect(names).toContain('helper_function');
    // impl methods
    expect(names.some(n => n.includes('new'))).toBe(true);
    expect(names.some(n => n.includes('run'))).toBe(true);
  });

  it('should extract imports', async () => {
    const result = await parseFile(FIXTURE, 'rust');
    expect(result.imports.length).toBeGreaterThanOrEqual(2);
    const stdIo = result.imports.find(i => i.source.includes('std::io'));
    expect(stdIo).toBeDefined();
  });

  it('should detect pub exports', async () => {
    const result = await parseFile(FIXTURE, 'rust');
    expect(result.exports).toContain('Engine');
    expect(result.exports).toContain('Status');
    expect(result.exports).toContain('Processable');
    expect(result.exports).toContain('public_function');
    expect(result.exports).not.toContain('helper_function');
  });

  it('should extract structs as classes', async () => {
    const result = await parseFile(FIXTURE, 'rust');
    expect(result.classes.some(c => c.name === 'Engine')).toBe(true);
  });

  it('should extract types (struct, enum, trait)', async () => {
    const result = await parseFile(FIXTURE, 'rust');
    expect(result.types.some(t => t.name === 'Engine' && t.kind === 'struct')).toBe(true);
    expect(result.types.some(t => t.name === 'Status' && t.kind === 'enum')).toBe(true);
    expect(result.types.some(t => t.name === 'Processable' && t.kind === 'trait')).toBe(true);
  });
});
