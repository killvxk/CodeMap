import path from 'path';
import fs from 'fs/promises';
import { loadGraph, loadMeta, saveGraph, computeFileHash } from '../graph.js';
import { traverseFiles, detectLanguage } from '../traverser.js';
import { initParser, parseFile } from '../parser.js';
import { detectModuleName } from '../scanner.js';
import { detectChangedFiles, mergeGraphUpdate } from '../differ.js';
import { saveSlices } from '../slicer.js';

/**
 * Try to get the current git commit hash from the given directory.
 * Returns null if the directory is not a git repo or simple-git is unavailable.
 *
 * @param {string} dir - Directory to check for git.
 * @returns {Promise<string|null>} The current commit hash or null.
 */
async function getGitCommitHash(dir) {
  try {
    const { simpleGit } = await import('simple-git');
    const git = simpleGit(dir);
    const isRepo = await git.checkIsRepo();
    if (!isRepo) return null;
    const log = await git.log({ maxCount: 1 });
    return log.latest ? log.latest.hash : null;
  } catch {
    return null;
  }
}

/**
 * Register the `update` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerUpdateCommand(program) {
  program
    .command('update [dir]')
    .description('Incrementally update the code graph for changed files')
    .action(async (dir) => {
      const rootDir = path.resolve(dir || '.');
      const outputDir = path.join(rootDir, '.codemap');

      // Step 1: Load existing graph and meta
      let graph, meta;
      try {
        graph = await loadGraph(outputDir);
        meta = await loadMeta(outputDir);
      } catch {
        console.error('No existing code graph found. Run "codegraph scan" first.');
        process.exit(1);
      }

      const startTime = Date.now();

      // Step 2: Initialize parser
      await initParser();

      // Step 3: Traverse files and compute current hashes
      const files = await traverseFiles(rootDir);
      const currentHashes = {};

      for (const absPath of files) {
        const language = detectLanguage(absPath);
        if (!language) continue;

        const relPath = path.relative(rootDir, absPath).replace(/\\/g, '/');
        try {
          const content = await fs.readFile(absPath, 'utf-8');
          currentHashes[relPath] = computeFileHash(content);
        } catch {
          // Skip files that can't be read
          continue;
        }
      }

      // Step 4: Detect changes
      const changes = detectChangedFiles(meta.fileHashes || {}, currentHashes);

      if (changes.added.length === 0 && changes.modified.length === 0 && changes.removed.length === 0) {
        console.log('No changes detected.');
        return;
      }

      // Step 5: Re-parse only changed/added files
      const updatedFiles = {};
      const changedPaths = [...changes.added, ...changes.modified];

      for (const relPath of changedPaths) {
        const absPath = path.resolve(rootDir, relPath);
        const language = detectLanguage(absPath);
        if (!language) continue;

        let parsed;
        try {
          parsed = await parseFile(absPath, language);
        } catch {
          continue;
        }

        const moduleName = detectModuleName(absPath, rootDir);
        const content = await fs.readFile(absPath, 'utf-8');
        const hash = computeFileHash(content);
        const baseName = path.basename(absPath, path.extname(absPath)).toLowerCase();
        const isEntryPoint = ['main', 'index', 'server', 'app', 'entry', 'bootstrap'].includes(baseName);

        updatedFiles[relPath] = {
          language,
          module: moduleName,
          hash,
          lines: parsed.lines,
          functions: parsed.functions,
          classes: parsed.classes,
          types: parsed.types,
          imports: parsed.imports,
          exports: parsed.exports,
          isEntryPoint,
        };
      }

      // Step 6: Merge updates into graph
      mergeGraphUpdate(graph, updatedFiles, changes.removed);

      // Step 7: Update metadata
      const commitHash = await getGitCommitHash(rootDir);
      graph.commitHash = commitHash;
      graph.scannedAt = new Date().toISOString();

      const scanDuration = Date.now() - startTime;

      // Build updated file hashes
      const fileHashes = {};
      for (const [filePath, fileData] of Object.entries(graph.files)) {
        fileHashes[filePath] = fileData.hash;
      }

      meta.lastScanAt = new Date().toISOString();
      meta.commitHash = commitHash;
      meta.scanDuration = scanDuration;
      meta.fileHashes = fileHashes;

      // Step 8: Save graph + meta + re-generate slices
      await saveGraph(outputDir, graph, meta);
      await saveSlices(outputDir, graph);

      // Step 9: Print change summary
      console.log(`Update complete.`);
      console.log(`  +${changes.added.length} ~${changes.modified.length} -${changes.removed.length}`);
      if (changes.added.length > 0) {
        console.log(`  Added: ${changes.added.join(', ')}`);
      }
      if (changes.modified.length > 0) {
        console.log(`  Modified: ${changes.modified.join(', ')}`);
      }
      if (changes.removed.length > 0) {
        console.log(`  Removed: ${changes.removed.join(', ')}`);
      }
    });
}
