import crypto from 'crypto';
import fs from 'fs/promises';
import path from 'path';

export function createEmptyGraph(projectName, rootDir) {
  return {
    version: '1.0',
    project: { name: projectName, root: rootDir },
    scannedAt: new Date().toISOString(),
    commitHash: null,
    config: { languages: [], excludePatterns: [] },
    summary: {
      totalFiles: 0, totalFunctions: 0, totalClasses: 0,
      languages: {}, modules: [], entryPoints: [],
    },
    modules: {},
    files: {},
  };
}

export function computeFileHash(content) {
  return 'sha256:' + crypto.createHash('sha256').update(content).digest('hex').slice(0, 16);
}

/** File basenames (without extension) that mark an entry point. */
const ENTRY_POINT_NAMES = new Set([
  'main', 'index', 'server', 'app', 'entry', 'bootstrap',
]);

/** Check whether a file path represents an entry point. */
export function isEntryPoint(filePath) {
  const baseName = path.basename(filePath, path.extname(filePath)).toLowerCase();
  return ENTRY_POINT_NAMES.has(baseName);
}

/**
 * Try to get the current git commit hash from the given directory.
 * Returns null if the directory is not a git repo or simple-git is unavailable.
 */
export async function getGitCommitHash(dir) {
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

export async function saveGraph(outputDir, graph, meta) {
  await fs.mkdir(outputDir, { recursive: true });
  await fs.writeFile(`${outputDir}/graph.json`, JSON.stringify(graph, null, 2), 'utf-8');
  await fs.writeFile(`${outputDir}/meta.json`, JSON.stringify(meta, null, 2), 'utf-8');
}

export async function loadGraph(outputDir) {
  const data = await fs.readFile(`${outputDir}/graph.json`, 'utf-8');
  return JSON.parse(data);
}

export async function loadMeta(outputDir) {
  const data = await fs.readFile(`${outputDir}/meta.json`, 'utf-8');
  return JSON.parse(data);
}
