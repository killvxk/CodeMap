import { describe, it, expect, beforeAll } from 'vitest';
import { analyzeImpact } from '../src/impact.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('impact analysis', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  describe('analyzeImpact', () => {
    it('should find module dependants (auth is depended on by api)', () => {
      const result = analyzeImpact(graph, 'auth');

      expect(result.targetType).toBe('module');
      expect(result.targetModule).toBe('auth');
      expect(result.directDependants).toContain('api');
      expect(result.impactedModules).toContain('auth');
      expect(result.impactedModules).toContain('api');
    });

    it('should find impacted files when targeting a file path', () => {
      // Find the actual file path for login.ts in the graph
      const loginFile = Object.keys(graph.files).find(f => f.includes('login.ts'));
      expect(loginFile).toBeDefined();

      const result = analyzeImpact(graph, loginFile);

      expect(result.targetType).toBe('file');
      expect(result.targetModule).toBe('auth');
      expect(result.impactedFiles.length).toBeGreaterThan(0);
      // Should include files from auth module and api module (since api depends on auth)
      const hasAuthFile = result.impactedFiles.some(f => f.includes('auth'));
      const hasApiFile = result.impactedFiles.some(f => f.includes('api'));
      expect(hasAuthFile).toBe(true);
      expect(hasApiFile).toBe(true);
    });

    it('should return empty dependants for a leaf module (api has no dependants)', () => {
      const result = analyzeImpact(graph, 'api');

      expect(result.targetType).toBe('module');
      expect(result.directDependants).toEqual([]);
      expect(result.transitiveDependants).toEqual([]);
      expect(result.impactedModules).toEqual(['api']);
    });

    it('should respect the depth limit', () => {
      const result = analyzeImpact(graph, 'auth', { depth: 0 });

      // With depth 0, no BFS should happen
      expect(result.transitiveDependants).toEqual([]);
      expect(result.impactedModules).toEqual(['auth']);
    });

    it('should handle unknown target gracefully', () => {
      const result = analyzeImpact(graph, 'nonExistentModule');

      expect(result.targetModule).toBe('nonExistentModule');
      expect(result.directDependants).toEqual([]);
      expect(result.transitiveDependants).toEqual([]);
    });
  });
});
