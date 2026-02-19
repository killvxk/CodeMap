import path from 'path';
import { scanProject } from '../scanner.js';
import { saveGraph, computeFileHash } from '../graph.js';
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
 * Register the `scan` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerScanCommand(program) {
  program
    .command('scan [dir]')
    .description('Scan a project directory and generate a code graph')
    .option('--exclude <patterns...>', 'Additional glob patterns to exclude')
    .action(async (dir, options) => {
      const rootDir = path.resolve(dir || '.');
      const outputDir = path.join(rootDir, '.codemap');

      const startTime = Date.now();

      // Step 1: Scan the project
      const graph = await scanProject(rootDir, {
        exclude: options.exclude || [],
      });

      // Step 2: Get git commit hash (gracefully skip)
      const commitHash = await getGitCommitHash(rootDir);
      graph.commitHash = commitHash;

      const scanDuration = Date.now() - startTime;

      // Step 3: Build file hashes map
      const fileHashes = {};
      for (const [filePath, fileData] of Object.entries(graph.files)) {
        fileHashes[filePath] = fileData.hash;
      }

      // Step 4: Build meta object
      const meta = {
        lastScanAt: new Date().toISOString(),
        commitHash,
        scanDuration,
        fileHashes,
      };

      // Step 5: Save graph and meta
      await saveGraph(outputDir, graph, meta);

      // Step 6: Save slices
      await saveSlices(outputDir, graph);

      // Step 7: Print summary
      const fileCount = graph.summary.totalFiles;
      const funcCount = graph.summary.totalFunctions;
      const moduleNames = graph.summary.modules.join(', ');

      console.log(`Scan complete.`);
      console.log(`  Files:     ${fileCount}`);
      console.log(`  Functions: ${funcCount}`);
      console.log(`  Modules:   ${moduleNames}`);
      console.log(`  Output:    ${outputDir}`);
    });
}
