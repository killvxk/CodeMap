import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../../src/parser.js';
import path from 'path';

const FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project/src/services/handler.go');

describe('Go adapter', () => {
  beforeAll(async () => { await initParser(); });

  it('should extract functions and methods', async () => {
    const result = await parseFile(FIXTURE, 'go');
    const names = result.functions.map(f => f.name);
    expect(names).toContain('NewHandler');
    expect(names).toContain('ServeHTTP');
    expect(names).toContain('internalHelper');
  });

  it('should extract imports', async () => {
    const result = await parseFile(FIXTURE, 'go');
    const sources = result.imports.map(i => i.source);
    expect(sources).toContain('fmt');
    expect(sources).toContain('net/http');
    expect(sources).toContain('encoding/json');
  });

  it('should detect exported names (capitalized)', async () => {
    const result = await parseFile(FIXTURE, 'go');
    expect(result.exports).toContain('NewHandler');
    expect(result.exports).toContain('Handler');
    expect(result.exports).toContain('Response');
    expect(result.exports).toContain('Servicer');
    expect(result.exports).not.toContain('internalHelper');
  });

  it('should extract structs as classes', async () => {
    const result = await parseFile(FIXTURE, 'go');
    const names = result.classes.map(c => c.name);
    expect(names).toContain('Handler');
    expect(names).toContain('Response');
  });

  it('should extract type declarations', async () => {
    const result = await parseFile(FIXTURE, 'go');
    const iface = result.types.find(t => t.name === 'Servicer');
    expect(iface).toBeDefined();
    expect(iface.kind).toBe('interface');
  });
});
