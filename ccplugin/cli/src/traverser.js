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

/** C++ file extensions (excluding .h which is shared with C). */
const CPP_EXTENSIONS = new Set(['.cpp', '.cc', '.cxx', '.hpp', '.hh']);

/**
 * Check whether a list of absolute file paths contains any C++ source files.
 * Used to decide if `.h` headers should be reclassified as C++.
 */
export function hasCppSourceFiles(files) {
  return files.some(f => CPP_EXTENSIONS.has(path.extname(f).toLowerCase()));
}

/**
 * Return the effective language for a file, applying `.h` → `cpp`
 * reclassification when the project contains C++ sources.
 */
export function effectiveLanguage(filePath, baseLanguage, projectHasCpp) {
  if (baseLanguage === 'c' && projectHasCpp && path.extname(filePath).toLowerCase() === '.h') {
    return 'cpp';
  }
  return baseLanguage;
}

export { LANGUAGE_EXTENSIONS, ALL_EXTENSIONS };
