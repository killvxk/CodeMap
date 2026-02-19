import { LanguageAdapter } from './base.js';

/**
 * Python language adapter.
 *
 * Walks a tree-sitter AST produced by the Python grammar and extracts
 * functions, imports, exports, classes and type declarations.
 */
export class PythonAdapter extends LanguageAdapter {
  constructor() {
    super('python');
  }

  // ---------------------------------------------------------------------------
  // Functions
  // ---------------------------------------------------------------------------

  /**
   * Collect all top-level function definitions, including those wrapped in
   * decorated_definition.  Methods inside classes are excluded.
   */
  extractFunctions(tree, sourceCode) {
    const functions = [];
    const root = tree.rootNode;

    for (let i = 0; i < root.namedChildCount; i++) {
      const node = root.namedChild(i);
      const funcNode = this._unwrapDecorated(node, 'function_definition');
      if (funcNode) {
        const fn = this._parseFunctionDefinition(funcNode, node);
        if (fn) functions.push(fn);
      }
    }

    return functions;
  }

  /**
   * Parse a function_definition node into a structured record.
   * @param {*} funcNode  - the function_definition node
   * @param {*} outerNode - the decorated_definition wrapper (or the funcNode itself)
   */
  _parseFunctionDefinition(funcNode, outerNode) {
    const nameNode = funcNode.childForFieldName('name');
    if (!nameNode) return null;

    const params = funcNode.childForFieldName('parameters');
    const returnType = funcNode.childForFieldName('return_type');

    const signature = this._buildSignature(nameNode.text, params, returnType);

    return {
      name: nameNode.text,
      signature,
      startLine: outerNode.startPosition.row + 1,
      endLine: outerNode.endPosition.row + 1,
    };
  }

  /**
   * Build a human-readable signature string: `name(params) -> return_type`
   */
  _buildSignature(name, paramsNode, returnTypeNode) {
    const params = paramsNode ? paramsNode.text : '()';
    const ret = returnTypeNode ? ` -> ${returnTypeNode.text}` : '';
    return `${name}${params}${ret}`;
  }

  // ---------------------------------------------------------------------------
  // Imports
  // ---------------------------------------------------------------------------

  extractImports(tree, _sourceCode) {
    const imports = [];

    this._walkNodes(tree.rootNode, (node) => {
      if (node.type === 'import_statement') {
        this._parseImportStatement(node, imports);
      } else if (node.type === 'import_from_statement') {
        this._parseImportFromStatement(node, imports);
      }
    });

    return imports;
  }

  /**
   * Handle `import os, sys` style imports.
   * Each dotted_name becomes a separate import entry.
   */
  _parseImportStatement(node, imports) {
    for (let i = 0; i < node.namedChildCount; i++) {
      const child = node.namedChild(i);
      if (child.type === 'dotted_name' || child.type === 'aliased_import') {
        const name = child.type === 'aliased_import'
          ? this._findChildOfType(child, 'dotted_name')?.text || child.firstNamedChild?.text
          : child.text;
        if (name) {
          imports.push({
            source: name,
            symbols: [name],
            isExternal: !name.startsWith('.'),
          });
        }
      }
    }
  }

  /**
   * Handle `from X import a, b` style imports.
   */
  _parseImportFromStatement(node, imports) {
    // Determine the module source.
    const moduleName = node.childForFieldName('module_name');
    let source = '';

    if (moduleName) {
      source = moduleName.text;
    } else {
      // Relative import with no module name: `from . import utils`
      // Collect the dots that precede the `import` keyword.
      for (let i = 0; i < node.childCount; i++) {
        const child = node.child(i);
        if (child.type === 'import') break; // keyword
        if (child.type === 'relative_import') {
          source = child.text;
          break;
        }
        if (child.type === '.') {
          source += '.';
        }
      }
    }

    // Collect imported symbols.
    const symbols = [];
    // Walk children looking for identifiers and aliased_import nodes after "import"
    let pastImport = false;
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i);
      if (child.type === 'import') {
        pastImport = true;
        continue;
      }
      if (!pastImport) continue;

      if (child.type === 'dotted_name' || child.type === 'identifier') {
        symbols.push(child.text);
      } else if (child.type === 'aliased_import') {
        const nameChild = child.firstNamedChild;
        if (nameChild) symbols.push(nameChild.text);
      } else if (child.type === 'wildcard_import') {
        symbols.push('*');
      }
    }

    if (source || symbols.length > 0) {
      imports.push({
        source,
        symbols,
        isExternal: !source.startsWith('.'),
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Exports
  // ---------------------------------------------------------------------------

  /**
   * In Python, exports are determined by:
   * 1. `__all__ = [...]` if present (explicit)
   * 2. Otherwise, all top-level function and class names (implicit)
   */
  extractExports(tree, sourceCode) {
    const allExports = this._extractDunderAll(tree);
    if (allExports) return allExports;

    // Fallback: collect all top-level function and class names.
    const exports = [];
    const root = tree.rootNode;
    for (let i = 0; i < root.namedChildCount; i++) {
      const node = root.namedChild(i);

      const funcNode = this._unwrapDecorated(node, 'function_definition');
      if (funcNode) {
        const nameNode = funcNode.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
        continue;
      }

      const classNode = this._unwrapDecorated(node, 'class_definition');
      if (classNode) {
        const nameNode = classNode.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }
    }
    return exports;
  }

  /**
   * Look for `__all__ = ["foo", "bar"]` at the top level and extract the names.
   * Returns null if no __all__ is found.
   */
  _extractDunderAll(tree) {
    const root = tree.rootNode;
    for (let i = 0; i < root.namedChildCount; i++) {
      const node = root.namedChild(i);
      if (node.type === 'expression_statement') {
        const expr = node.firstNamedChild;
        if (expr && expr.type === 'assignment') {
          const left = expr.childForFieldName('left');
          if (left && left.text === '__all__') {
            return this._extractListStrings(expr.childForFieldName('right'));
          }
        }
      }
      // Direct assignment at module level
      if (node.type === 'assignment') {
        const left = node.childForFieldName('left');
        if (left && left.text === '__all__') {
          return this._extractListStrings(node.childForFieldName('right'));
        }
      }
    }
    return null;
  }

  /**
   * Extract string values from a list node: `["foo", "bar"]` -> `["foo", "bar"]`
   */
  _extractListStrings(listNode) {
    if (!listNode || listNode.type !== 'list') return null;
    const strings = [];
    for (let i = 0; i < listNode.namedChildCount; i++) {
      const elem = listNode.namedChild(i);
      if (elem.type === 'string') {
        strings.push(this._stripQuotes(elem.text));
      }
    }
    return strings;
  }

  // ---------------------------------------------------------------------------
  // Classes
  // ---------------------------------------------------------------------------

  /**
   * Collect all top-level class definitions, including those wrapped in
   * decorated_definition.
   */
  extractClasses(tree, _sourceCode) {
    const classes = [];
    const root = tree.rootNode;

    for (let i = 0; i < root.namedChildCount; i++) {
      const node = root.namedChild(i);
      const classNode = this._unwrapDecorated(node, 'class_definition');
      if (classNode) {
        const nameNode = classNode.childForFieldName('name');
        if (nameNode) {
          classes.push({
            name: nameNode.text,
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }
    }

    return classes;
  }

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

  /** Python has no first-class type declarations in the TypeScript sense. */
  extractTypes(_tree, _sourceCode) {
    return [];
  }

  // ---------------------------------------------------------------------------
  // Helpers
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
      // Push children in reverse so that leftmost children are visited first
      for (let i = node.childCount - 1; i >= 0; i--) {
        stack.push(node.child(i));
      }
    }
  }

  /**
   * If `node` is a `decorated_definition` wrapping the expected inner type,
   * return the inner node.  If `node` itself is the expected type, return it.
   * Otherwise return null.
   */
  _unwrapDecorated(node, expectedType) {
    if (node.type === expectedType) return node;
    if (node.type === 'decorated_definition') {
      for (let i = 0; i < node.namedChildCount; i++) {
        const child = node.namedChild(i);
        if (child.type === expectedType) return child;
      }
    }
    return null;
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
