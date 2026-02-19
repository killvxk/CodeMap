import fg from 'fast-glob';
import path from 'path';

const DEFAULT_EXCLUDE = [
  '**/node_modules/**',
  '**/dist/**',
  '**/build/**',
  '**/.git/**',
  '**/vendor/**',
  '**/__pycache__/**',
  '**/target/**',
  '**/.codemap/**',
];

const LANGUAGE_EXTENSIONS = {
  typescript: ['.ts', '.tsx'],
  javascript: ['.js', '.jsx', '.mjs', '.cjs'],
  python: ['.py'],
  go: ['.go'],
  rust: ['.rs'],
  java: ['.java'],
  c: ['.c', '.h'],
  cpp: ['.cpp', '.cc', '.cxx', '.hpp', '.hh'],
};

const ALL_EXTENSIONS = Object.values(LANGUAGE_EXTENSIONS).flat();

export function detectLanguage(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  for (const [lang, exts] of Object.entries(LANGUAGE_EXTENSIONS)) {
    if (exts.includes(ext)) return lang;
  }
  return null;
}

export async function traverseFiles(rootDir, options = {}) {
  const { extensions = ALL_EXTENSIONS, exclude = [] } = options;
  const patterns = extensions.map(ext => `**/*${ext}`);
  const ignorePatterns = [...DEFAULT_EXCLUDE, ...exclude];

  const files = await fg(patterns, {
    cwd: rootDir,
    absolute: true,
    ignore: ignorePatterns,
    dot: false,
  });

  return files.sort();
}

export { LANGUAGE_EXTENSIONS, ALL_EXTENSIONS };
