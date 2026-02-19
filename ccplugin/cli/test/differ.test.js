import { describe, it, expect } from 'vitest';
import { detectChangedFiles, mergeGraphUpdate } from '../src/differ.js';

describe('differ', () => {
  describe('detectChangedFiles', () => {
    it('should correctly identify added, modified, removed, and unchanged files', () => {
      const oldHashes = {
        'src/auth/login.ts': 'sha256:aaa',
        'src/api/routes.ts': 'sha256:bbb',
        'src/old/removed.ts': 'sha256:ccc',
      };
      const newHashes = {
        'src/auth/login.ts': 'sha256:aaa',  // unchanged
        'src/api/routes.ts': 'sha256:xxx',  // modified (hash changed)
        'src/new/added.ts': 'sha256:ddd',   // added
      };

      const result = detectChangedFiles(oldHashes, newHashes);

      expect(result.added).toEqual(['src/new/added.ts']);
      expect(result.modified).toEqual(['src/api/routes.ts']);
      expect(result.removed).toEqual(['src/old/removed.ts']);
      expect(result.unchanged).toEqual(['src/auth/login.ts']);
    });

    it('should return all empty arrays when hashes are identical', () => {
      const hashes = {
        'src/a.ts': 'sha256:aaa',
        'src/b.ts': 'sha256:bbb',
      };

      const result = detectChangedFiles(hashes, { ...hashes });
      expect(result.added).toEqual([]);
      expect(result.modified).toEqual([]);
      expect(result.removed).toEqual([]);
      expect(result.unchanged).toEqual(['src/a.ts', 'src/b.ts']);
    });

    it('should handle empty old hashes (fresh scan)', () => {
      const newHashes = {
        'src/a.ts': 'sha256:aaa',
        'src/b.ts': 'sha256:bbb',
      };

      const result = detectChangedFiles({}, newHashes);
      expect(result.added).toEqual(['src/a.ts', 'src/b.ts']);
      expect(result.modified).toEqual([]);
      expect(result.removed).toEqual([]);
      expect(result.unchanged).toEqual([]);
    });
  });

  describe('mergeGraphUpdate', () => {
    it('should correctly add, update, and remove files in the graph', () => {
      const graph = {
        modules: {
          auth: {
            files: ['src/auth/login.ts'],
            dependsOn: [],
            dependedBy: ['api'],
          },
          api: {
            files: ['src/api/routes.ts'],
            dependsOn: ['auth'],
            dependedBy: [],
          },
          old: {
            files: ['src/old/removed.ts'],
            dependsOn: [],
            dependedBy: [],
          },
        },
        files: {
          'src/auth/login.ts': {
            language: 'typescript',
            module: 'auth',
            hash: 'sha256:aaa',
            lines: 10,
            functions: [{ name: 'login' }],
            classes: [],
            types: [],
            imports: [],
            exports: ['login'],
          },
          'src/api/routes.ts': {
            language: 'typescript',
            module: 'api',
            hash: 'sha256:bbb',
            lines: 5,
            functions: [{ name: 'handleLogin' }],
            classes: [],
            types: [],
            imports: [],
            exports: ['handleLogin'],
          },
          'src/old/removed.ts': {
            language: 'typescript',
            module: 'old',
            hash: 'sha256:ccc',
            lines: 3,
            functions: [],
            classes: [],
            types: [],
            imports: [],
            exports: [],
          },
        },
        summary: {
          totalFiles: 3,
          totalFunctions: 2,
          totalClasses: 0,
          languages: { typescript: 3 },
          modules: ['api', 'auth', 'old'],
        },
      };

      const updatedFiles = {
        'src/api/routes.ts': {
          language: 'typescript',
          module: 'api',
          hash: 'sha256:xxx',
          lines: 8,
          functions: [{ name: 'handleLogin' }, { name: 'handleLogout' }],
          classes: [],
          types: [],
          imports: [],
          exports: ['handleLogin', 'handleLogout'],
        },
        'src/new/added.ts': {
          language: 'typescript',
          module: 'newmod',
          hash: 'sha256:ddd',
          lines: 15,
          functions: [{ name: 'newFunc' }],
          classes: [{ name: 'NewClass' }],
          types: [],
          imports: [],
          exports: ['newFunc', 'NewClass'],
        },
      };

      const removedFiles = ['src/old/removed.ts'];

      mergeGraphUpdate(graph, updatedFiles, removedFiles);

      // Removed file should be gone
      expect(graph.files['src/old/removed.ts']).toBeUndefined();
      expect(graph.modules['old']).toBeUndefined(); // empty module removed

      // Updated file should have new data
      expect(graph.files['src/api/routes.ts'].hash).toBe('sha256:xxx');
      expect(graph.files['src/api/routes.ts'].functions).toHaveLength(2);

      // Added file should exist
      expect(graph.files['src/new/added.ts']).toBeDefined();
      expect(graph.modules['newmod']).toBeDefined();
      expect(graph.modules['newmod'].files).toContain('src/new/added.ts');

      // Summary should be recalculated
      expect(graph.summary.totalFiles).toBe(3); // login + routes + added
      expect(graph.summary.totalFunctions).toBe(4); // login + handleLogin + handleLogout + newFunc
      expect(graph.summary.totalClasses).toBe(1); // NewClass
      expect(graph.summary.modules).toContain('newmod');
      expect(graph.summary.modules).not.toContain('old');
    });

    it('should handle removing files that do not exist gracefully', () => {
      const graph = {
        modules: {
          auth: { files: ['a.ts'], dependsOn: [], dependedBy: [] },
        },
        files: {
          'a.ts': {
            language: 'typescript',
            module: 'auth',
            hash: 'sha256:aaa',
            lines: 1,
            functions: [],
            classes: [],
            types: [],
            imports: [],
            exports: [],
          },
        },
        summary: {
          totalFiles: 1,
          totalFunctions: 0,
          totalClasses: 0,
          languages: { typescript: 1 },
          modules: ['auth'],
        },
      };

      // Removing a non-existent file should not throw
      mergeGraphUpdate(graph, {}, ['nonexistent.ts']);

      // Graph should remain unchanged
      expect(graph.summary.totalFiles).toBe(1);
      expect(graph.files['a.ts']).toBeDefined();
    });
  });
});
