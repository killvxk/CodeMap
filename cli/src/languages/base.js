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

  // ---------------------------------------------------------------------------
  // Shared helpers
  // ---------------------------------------------------------------------------

  /**
   * Walk all nodes depth-first, calling `visitor(node)` for each.
   * Uses an explicit stack (not the cursor API) for simplicity and reliability.
   */
  _walkNodes(root, visitor) {
    const stack = [root];
    while (stack.length > 0) {
      const node = stack.pop();
      visitor(node);
      for (let i = node.childCount - 1; i >= 0; i--) {
        stack.push(node.child(i));
      }
    }
  }

  /** Find the first direct child with the given type. */
  _findChildOfType(node, type) {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i);
      if (child.type === type) return child;
    }
    return null;
  }

  /** Strip surrounding quotes from a string literal. */
  _stripQuotes(text) {
    return text.replace(/^['"`]|['"`]$/g, '');
  }
}
