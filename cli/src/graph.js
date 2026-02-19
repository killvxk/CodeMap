import crypto from 'crypto';
import fs from 'fs/promises';

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
