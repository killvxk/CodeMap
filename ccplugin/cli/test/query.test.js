import { describe, it, expect, beforeAll } from 'vitest';
import { querySymbol, queryModule, queryDependants, queryDependencies } from '../src/query.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('query engine', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  describe('querySymbol', () => {
    it('should find a function by name (login)', () => {
      const results = querySymbol(graph, 'login');
      expect(results.length).toBeGreaterThan(0);

      const loginFn = results.find(r => r.kind === 'function' && r.name === 'login');
      expect(loginFn).toBeDefined();
      expect(loginFn.module).toBe('auth');
      expect(loginFn.lines.start).toBeDefined();
      expect(loginFn.lines.end).toBeDefined();
    });

    it('should filter by type when option is provided', () => {
      const funcResults = querySymbol(graph, 'login', { type: 'function' });
      const typeResults = querySymbol(graph, 'login', { type: 'type' });

      // login is an exported function, not a type
      const loginFn = funcResults.find(r => r.name === 'login' && r.kind === 'function');
      expect(loginFn).toBeDefined();

      // LoginOptions is a type/interface, not a function
      const loginType = typeResults.find(r => r.name === 'login' && r.kind === 'function');
      expect(loginType).toBeUndefined();
    });

    it('should return an empty array for unknown symbol', () => {
      const results = querySymbol(graph, 'nonExistentSymbol12345');
      expect(results).toEqual([]);
    });
  });

  describe('queryModule', () => {
    it('should return module info for existing module (auth)', () => {
      const result = queryModule(graph, 'auth');
      expect(result).not.toBeNull();
      expect(result.name).toBe('auth');
      expect(result.files).toBeInstanceOf(Array);
      expect(result.files.length).toBeGreaterThan(0);
      expect(result.dependsOn).toBeInstanceOf(Array);
      expect(result.dependedBy).toBeInstanceOf(Array);
    });

    it('should return null for unknown module', () => {
      const result = queryModule(graph, 'nonExistentModule');
      expect(result).toBeNull();
    });
  });

  describe('queryDependants', () => {
    it('should return dependants of auth module', () => {
      const dependants = queryDependants(graph, 'auth');
      expect(dependants).toBeInstanceOf(Array);
      expect(dependants).toContain('api');
    });

    it('should return empty array for unknown module', () => {
      const dependants = queryDependants(graph, 'nonExistentModule');
      expect(dependants).toEqual([]);
    });
  });

  describe('queryDependencies', () => {
    it('should return dependencies of api module', () => {
      const deps = queryDependencies(graph, 'api');
      expect(deps).toBeInstanceOf(Array);
      expect(deps).toContain('auth');
    });

    it('should return empty array for unknown module', () => {
      const deps = queryDependencies(graph, 'nonExistentModule');
      expect(deps).toEqual([]);
    });
  });
});
