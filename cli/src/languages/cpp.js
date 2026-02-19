import { LanguageAdapter } from './base.js';

/**
 * C / C++ language adapter.
 *
 * Walks a tree-sitter AST produced by either the C or C++ grammar and extracts
 * functions, #include imports, exports (non-static symbols), classes/structs,
 * and type declarations (struct, class, enum, typedef, namespace).
 *
 * A single adapter class handles both C and C++ because the C++ grammar is a
 * superset of C, and the extraction logic is largely identical.
 */
export class CppAdapter extends LanguageAdapter {
  constructor() {
    super('cpp');
  }

  // ---------------------------------------------------------------------------
  // Functions
  // ---------------------------------------------------------------------------

  /**
   * Extract function definitions.
   *
   * In C/C++ the AST node is `function_definition`.  Its `declarator` field
   * is a `function_declarator` whose own `declarator` gives the function name
   * (possibly qualified with `::` in C++).
   */
  extractFunctions(tree, sourceCode) {
    const functions = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type === 'function_definition') {
        const fn = this._parseFunctionDef(node, sourceCode);
        if (fn) functions.push(fn);
      }
    });
    return functions;
  }

  /**
   * Parse a function_definition node into a structured record.
   *
   * The signature is reconstructed from the return type, declarator text
   * (which includes the name and parameter list), and any trailing qualifiers.
   */
  _parseFunctionDef(node, sourceCode) {
    // Find the function_declarator – may be nested inside pointer_declarator
    // or reference_declarator.
    const funcDeclarator = this._findDescendantOfType(node, 'function_declarator');
    if (!funcDeclarator) return null;

    const nameNode = funcDeclarator.childForFieldName('declarator');
    if (!nameNode) return null;

    const name = nameNode.text;

    // Build signature from everything before the body
    const body = node.childForFieldName('body');
    let signature;
    if (body) {
      const sigEnd = body.startIndex;
      signature = sourceCode.slice(node.startIndex, sigEnd).trim();
    } else {
      signature = sourceCode.slice(node.startIndex, node.endIndex).trim();
    }

    return {
      name,
      signature,
      startLine: node.startPosition.row + 1,
      endLine: node.endPosition.row + 1,
    };
  }

  // ---------------------------------------------------------------------------
  // Imports (#include directives)
  // ---------------------------------------------------------------------------

  /**
   * Extract `#include` directives.
   *
   * - `#include <header>` → system_lib_string → isExternal: true
   * - `#include "header"` → string_literal    → isExternal: false
   */
  extractImports(tree, _sourceCode) {
    const imports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'preproc_include') return;

      const pathNode = this._findChildOfType(node, 'system_lib_string')
        || this._findChildOfType(node, 'string_literal');
      if (!pathNode) return;

      const isExternal = pathNode.type === 'system_lib_string';
      // Strip < > or " "
      const raw = pathNode.text.replace(/^[<"]|[>"]$/g, '');

      imports.push({
        source: raw,
        symbols: [],
        isExternal,
      });
    });
    return imports;
  }

  // ---------------------------------------------------------------------------
  // Exports (non-static function and type names)
  // ---------------------------------------------------------------------------

  /**
   * In C/C++ there is no `export` keyword (ignoring C++20 modules).
   * By convention, all non-static functions and all type names at file /
   * namespace scope are considered "exported".
   */
  extractExports(tree, _sourceCode) {
    const exports = [];

    this._walkNodes(tree.rootNode, (node) => {
      // Non-static function definitions
      if (node.type === 'function_definition') {
        if (this._hasStorageClassStatic(node)) return;
        const funcDeclarator = this._findDescendantOfType(node, 'function_declarator');
        if (!funcDeclarator) return;
        const nameNode = funcDeclarator.childForFieldName('declarator');
        if (nameNode) {
          // Use the bare identifier (strip qualified names like Engine::start → start)
          const name = this._bareIdentifier(nameNode.text);
          exports.push(name);
        }
      }

      // Struct / class / enum type names
      if (
        node.type === 'struct_specifier' ||
        node.type === 'class_specifier' ||
        node.type === 'enum_specifier'
      ) {
        const nameNode = node.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }

      // Typedefs
      if (node.type === 'type_definition') {
        const declarator = this._findDescendantOfType(node, 'type_identifier');
        if (declarator) exports.push(declarator.text);
      }
    });

    // Deduplicate (e.g. a struct may appear as both struct_specifier and in a typedef)
    return [...new Set(exports)];
  }

  // ---------------------------------------------------------------------------
  // Classes (C++ classes and C structs)
  // ---------------------------------------------------------------------------

  /**
   * Map C++ `class_specifier` and C/C++ `struct_specifier` nodes to the
   * "classes" output.  Only named specifiers with a body are included (forward
   * declarations are skipped).
   */
  extractClasses(tree, _sourceCode) {
    const classes = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'class_specifier' && node.type !== 'struct_specifier') return;

      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;

      const body = node.childForFieldName('body');
      if (!body) return; // skip forward declarations

      classes.push({
        name: nameNode.text,
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
    });
    return classes;
  }

  // ---------------------------------------------------------------------------
  // Types (struct, class, enum, typedef, namespace)
  // ---------------------------------------------------------------------------

  extractTypes(tree, _sourceCode) {
    const types = [];
    this._walkNodes(tree.rootNode, (node) => {
      // struct
      if (node.type === 'struct_specifier') {
        const nameNode = node.childForFieldName('name');
        const body = node.childForFieldName('body');
        if (nameNode && body) {
          types.push({
            name: nameNode.text,
            kind: 'struct',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }

      // class (C++ only)
      if (node.type === 'class_specifier') {
        const nameNode = node.childForFieldName('name');
        const body = node.childForFieldName('body');
        if (nameNode && body) {
          types.push({
            name: nameNode.text,
            kind: 'class',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }

      // enum (including C++ enum class)
      if (node.type === 'enum_specifier') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
          types.push({
            name: nameNode.text,
            kind: 'enum',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }

      // typedef
      if (node.type === 'type_definition') {
        const declarator = this._findDescendantOfType(node, 'type_identifier');
        if (declarator) {
          types.push({
            name: declarator.text,
            kind: 'typedef',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }

      // namespace (C++ only)
      if (node.type === 'namespace_definition') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
          types.push({
            name: nameNode.text,
            kind: 'namespace',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }
    });
    return types;
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /** Find the first descendant (BFS) with the given type. */
  _findDescendantOfType(node, type) {
    const queue = [];
    for (let i = 0; i < node.childCount; i++) {
      queue.push(node.child(i));
    }
    while (queue.length > 0) {
      const current = queue.shift();
      if (current.type === type) return current;
      for (let i = 0; i < current.childCount; i++) {
        queue.push(current.child(i));
      }
    }
    return null;
  }

  /**
   * Check whether a function_definition has the `static` storage class.
   *
   * In tree-sitter-c/cpp, the storage class specifier appears as a direct
   * child of the function_definition with type `storage_class_specifier`
   * whose text is "static".
   */
  _hasStorageClassStatic(funcDefNode) {
    for (let i = 0; i < funcDefNode.childCount; i++) {
      const child = funcDefNode.child(i);
      if (child.type === 'storage_class_specifier' && child.text === 'static') {
        return true;
      }
    }
    return false;
  }

  /**
   * Extract the bare identifier from a possibly qualified name.
   * e.g. "Engine::start" → "start", "initialize" → "initialize"
   */
  _bareIdentifier(text) {
    const idx = text.lastIndexOf('::');
    return idx >= 0 ? text.slice(idx + 2) : text;
  }
}
