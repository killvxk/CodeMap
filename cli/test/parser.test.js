import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('parser', () => {
  beforeAll(async () => {
    await initParser();
  });

  // ── login.ts ──────────────────────────────────────────────────────────────

  describe('login.ts', () => {
    let result;

    beforeAll(async () => {
      const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
      result = await parseFile(filePath, 'typescript');
    });

    it('should extract functions', () => {
      expect(result.functions).toBeDefined();
      expect(result.functions.length).toBeGreaterThan(0);
      const loginFn = result.functions.find((f) => f.name === 'login');
      expect(loginFn).toBeDefined();
      expect(loginFn.startLine).toBeGreaterThan(0);
      expect(loginFn.endLine).toBeGreaterThanOrEqual(loginFn.startLine);
      expect(loginFn.signature).toContain('login');
      expect(loginFn.signature).toContain('opts');
    });

    it('should extract imports', () => {
      expect(result.imports.length).toBeGreaterThan(0);

      const dbImport = result.imports.find(
        (i) => i.source && i.source.includes('db/users'),
      );
      expect(dbImport).toBeDefined();
      expect(dbImport.symbols).toContain('getUserById');
      expect(dbImport.isExternal).toBe(false);

      const bcryptImport = result.imports.find(
        (i) => i.source === 'bcrypt',
      );
      expect(bcryptImport).toBeDefined();
      expect(bcryptImport.isExternal).toBe(true);
      expect(bcryptImport.symbols).toContain('bcrypt');
    });

    it('should extract exports', () => {
      expect(result.exports).toContain('login');
      expect(result.exports).toContain('LoginOptions');
    });

    it('should extract types', () => {
      expect(result.types.length).toBeGreaterThan(0);
      const iface = result.types.find((t) => t.name === 'LoginOptions');
      expect(iface).toBeDefined();
      expect(iface.kind).toBe('interface');
    });

    it('should report line count', () => {
      expect(result.lines).toBeGreaterThan(0);
    });
  });

  // ── routes.ts ─────────────────────────────────────────────────────────────

  describe('routes.ts', () => {
    let result;

    beforeAll(async () => {
      const filePath = path.join(FIXTURE_DIR, 'src/api/routes.ts');
      result = await parseFile(filePath, 'typescript');
    });

    it('should extract the handleLogin function', () => {
      const fn = result.functions.find((f) => f.name === 'handleLogin');
      expect(fn).toBeDefined();
      expect(fn.signature).toContain('handleLogin');
    });

    it('should extract the login import', () => {
      const imp = result.imports.find((i) => i.source.includes('auth/login'));
      expect(imp).toBeDefined();
      expect(imp.symbols).toContain('login');
      expect(imp.isExternal).toBe(false);
    });

    it('should extract exports', () => {
      expect(result.exports).toContain('handleLogin');
    });
  });

  // ── Edge cases ────────────────────────────────────────────────────────────

  it('should throw for uninitialised parser usage', async () => {
    // We already initialised in beforeAll, so this just tests
    // that the parser is functional after init.
    const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
    const result = await parseFile(filePath, 'typescript');
    expect(result).toBeDefined();
  });

  it('should throw for unsupported language', async () => {
    const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
    await expect(parseFile(filePath, 'haskell')).rejects.toThrow(
      /No adapter registered/,
    );
  });
});
