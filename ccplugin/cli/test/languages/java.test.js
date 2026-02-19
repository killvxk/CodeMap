import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../../src/parser.js';
import path from 'path';

const FIXTURE = path.resolve(import.meta.dirname, '../fixtures/sample-project/src/models/User.java');

describe('Java adapter', () => {
  beforeAll(async () => { await initParser(); });

  it('should extract methods', async () => {
    const result = await parseFile(FIXTURE, 'java');
    const names = result.functions.map(f => f.name);
    expect(names.some(n => n.includes('getName'))).toBe(true);
    expect(names.some(n => n.includes('setEmail'))).toBe(true);
  });

  it('should extract imports', async () => {
    const result = await parseFile(FIXTURE, 'java');
    const sources = result.imports.map(i => i.source);
    expect(sources.some(s => s.includes('java.util'))).toBe(true);
  });

  it('should extract public exports', async () => {
    const result = await parseFile(FIXTURE, 'java');
    expect(result.exports).toContain('User');
  });

  it('should extract classes', async () => {
    const result = await parseFile(FIXTURE, 'java');
    expect(result.classes.some(c => c.name === 'User')).toBe(true);
  });

  it('should extract types (class, interface, enum)', async () => {
    const result = await parseFile(FIXTURE, 'java');
    expect(result.types.some(t => t.name === 'User' && t.kind === 'class')).toBe(true);
    expect(result.types.some(t => t.name === 'UserService' && t.kind === 'interface')).toBe(true);
    expect(result.types.some(t => t.name === 'UserRole' && t.kind === 'enum')).toBe(true);
  });
});
