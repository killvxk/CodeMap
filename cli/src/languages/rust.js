import { LanguageAdapter } from './base.js';

/**
 * Rust language adapter.
 *
 * Walks a tree-sitter AST produced by the Rust grammar and extracts
 * functions (including impl methods), imports, exports (pub items),
 * classes (structs) and type declarations (struct, enum, trait, type).
 */
export class RustAdapter extends LanguageAdapter {
  constructor() {
    super('rust');
  }

  // ---------------------------------------------------------------------------
  // Functions
  // ---------------------------------------------------------------------------

  /**
   * Extract function declarations:
   * - Top-level `function_item` nodes
   * - Methods inside `impl_item` > `declaration_list` (prefixed with impl type)
   */
  extractFunctions(tree, sourceCode) {
    const functions = [];

    this._walkNodes(tree.rootNode, (node) => {
      if (node.type === 'function_item') {
        const nameNode = node.childForFieldName('name');
        if (!nameNode) return;

        const implType = this._getImplType(node);
        const name = implType ? `${implType}::${nameNode.text}` : nameNode.text;
        const signature = this._buildFnSignature(node, name, sourceCode);

        functions.push({
          name,
          signature,
          startLine: node.startPosition.row + 1,
          endLine: node.endPosition.row + 1,
        });
      }
    });

    return functions;
  }

  /**
   * If a function_item is inside an impl_item, return the impl type name.
   * Traverses upward through declaration_list to find the enclosing impl_item.
   */
  _getImplType(node) {
    let current = node.parent;
    while (current) {
      if (current.type === 'impl_item') {
        const typeNode = current.childForFieldName('type');
        if (typeNode) return typeNode.text;
        return null;
      }
      current = current.parent;
    }
    return null;
  }

  /**
   * Build a human-readable function signature from a function_item node.
   * Format: `name(params) -> return_type`
   */
  _buildFnSignature(node, name, _sourceCode) {
    const params = node.childForFieldName('parameters');
    const retType = node.childForFieldName('return_type');
    let sig = `${name}${params ? params.text : '()'}`;
    if (retType) {
      sig += ` ${retType.text}`;
    }
    return sig;
  }

  // ---------------------------------------------------------------------------
  // Imports
  // ---------------------------------------------------------------------------

  /**
   * Extract `use_declaration` nodes.
   * Parses the use path to determine source and imported symbols.
   */
  extractImports(tree, _sourceCode) {
    const imports = [];

    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'use_declaration') return;

      const result = { source: '', symbols: [], isExternal: true };
      this._parseUseTree(node, result);

      if (result.source) {
        result.isExternal = this._isExternalImport(result.source);
        imports.push(result);
      }
    });

    return imports;
  }

  /**
   * Recursively parse the children of a use_declaration to extract
   * the source path and imported symbols.
   */
  _parseUseTree(node, result) {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i);

      if (child.type === 'scoped_identifier' || child.type === 'scoped_use_list') {
        // For scoped_use_list like `std::io::{Read, Write}`,
        // the path is everything before the use_list.
        const pathPart = child.childForFieldName('path');
        const namePart = child.childForFieldName('name');
        const listPart = this._findChildOfType(child, 'use_list');

        if (pathPart) {
          result.source = pathPart.text;
        }

        if (listPart) {
          // e.g. {Read, Write} or {self, Read, Write}
          this._extractUseListSymbols(listPart, result.symbols);
        } else if (namePart) {
          result.symbols.push(namePart.text);
        }
        return; // done
      }

      if (child.type === 'identifier') {
        // Simple `use foo;`
        result.source = child.text;
        result.symbols.push(child.text);
        return;
      }

      if (child.type === 'use_list') {
        this._extractUseListSymbols(child, result.symbols);
      }
    }
  }

  /**
   * Extract symbol names from a use_list node (`{Read, Write, self}`).
   */
  _extractUseListSymbols(listNode, symbols) {
    for (let i = 0; i < listNode.namedChildCount; i++) {
      const child = listNode.namedChild(i);
      if (child.type === 'identifier' || child.type === 'self') {
        symbols.push(child.text);
      } else if (child.type === 'scoped_identifier') {
        // Nested path like `io::Read` inside a use list
        const namePart = child.childForFieldName('name');
        if (namePart) symbols.push(namePart.text);
      }
    }
  }

  /**
   * Determine if an import source is external.
   * Internal imports start with `crate`, `self`, or `super`.
   */
  _isExternalImport(source) {
    return (
      !source.startsWith('crate') &&
      !source.startsWith('self') &&
      !source.startsWith('super')
    );
  }

  // ---------------------------------------------------------------------------
  // Exports
  // ---------------------------------------------------------------------------

  /**
   * Collect names of items that have a `visibility_modifier` child containing `pub`.
   */
  extractExports(tree, _sourceCode) {
    const exports = [];

    this._walkNodes(tree.rootNode, (node) => {
      if (!this._hasPubVisibility(node)) return;

      let nameNode = null;

      switch (node.type) {
        case 'function_item':
          nameNode = node.childForFieldName('name');
          break;
        case 'struct_item':
        case 'enum_item':
        case 'trait_item':
        case 'type_item':
          nameNode = node.childForFieldName('name');
          break;
        case 'mod_item':
          nameNode = node.childForFieldName('name');
          break;
        default:
          break;
      }

      // Only export top-level pub items (not methods inside impl blocks)
      if (nameNode && !this._isInsideImpl(node)) {
        exports.push(nameNode.text);
      }
    });

    return exports;
  }

  /**
   * Check if a node has a visibility_modifier child that contains `pub`.
   */
  _hasPubVisibility(node) {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i);
      if (child.type === 'visibility_modifier' && child.text.includes('pub')) {
        return true;
      }
    }
    return false;
  }

  /**
   * Check if a node is inside an impl_item (i.e. it's a method, not a top-level item).
   */
  _isInsideImpl(node) {
    let current = node.parent;
    while (current) {
      if (current.type === 'impl_item') return true;
      current = current.parent;
    }
    return false;
  }

  // ---------------------------------------------------------------------------
  // Classes (structs)
  // ---------------------------------------------------------------------------

  extractClasses(tree, _sourceCode) {
    const classes = [];

    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'struct_item') return;
      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;
      classes.push({
        name: nameNode.text,
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
    });

    return classes;
  }

  // ---------------------------------------------------------------------------
  // Types (struct, enum, trait, type)
  // ---------------------------------------------------------------------------

  extractTypes(tree, _sourceCode) {
    const types = [];
    const typeMap = {
      struct_item: 'struct',
      enum_item: 'enum',
      trait_item: 'trait',
      type_item: 'type',
    };

    this._walkNodes(tree.rootNode, (node) => {
      const kind = typeMap[node.type];
      if (!kind) return;
      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;
      types.push({
        name: nameNode.text,
        kind,
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
    });

    return types;
  }

}
