/**
 * Base class for language adapters.
 *
 * Each language adapter knows how to walk a tree-sitter AST produced by a
 * specific grammar and extract structural information (functions, imports,
 * exports, classes, type declarations).
 */
export class LanguageAdapter {
  /**
   * @param {string} language - language identifier, e.g. "typescript"
   */
  constructor(language) {
    this.language = language;
  }

  /**
   * Extract function declarations from the AST.
   * @param {import('web-tree-sitter').Tree} tree
   * @param {string} sourceCode
   * @returns {Array<{name: string, signature: string, startLine: number, endLine: number}>}
   */
  extractFunctions(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  /**
   * Extract import statements from the AST.
   * @param {import('web-tree-sitter').Tree} tree
   * @param {string} sourceCode
   * @returns {Array<{source: string, symbols: string[], isExternal: boolean}>}
   */
  extractImports(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  /**
   * Extract exported names from the AST.
   * @param {import('web-tree-sitter').Tree} tree
   * @param {string} sourceCode
   * @returns {string[]}
   */
  extractExports(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  /**
   * Extract class declarations from the AST.
   * @param {import('web-tree-sitter').Tree} tree
   * @param {string} sourceCode
   * @returns {Array<{name: string, startLine: number, endLine: number}>}
   */
  extractClasses(tree, sourceCode) {
    return [];
  }

  /**
   * Extract type / interface declarations from the AST.
   * @param {import('web-tree-sitter').Tree} tree
   * @param {string} sourceCode
   * @returns {Array<{name: string, kind: string, startLine: number, endLine: number}>}
   */
  extractTypes(tree, sourceCode) {
    return [];
  }
}
