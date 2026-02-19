import { LanguageAdapter } from './base.js';

/**
 * Go language adapter.
 *
 * Walks a tree-sitter AST produced by the Go grammar and extracts
 * functions, imports, exports (capitalized identifiers), structs
 * (mapped to "classes") and type declarations.
 */
export class GoAdapter extends LanguageAdapter {
  constructor() {
    super('go');
  }

  // ---------------------------------------------------------------------------
  // Functions
  // ---------------------------------------------------------------------------

  /**
   * Extract function_declaration and method_declaration nodes.
   *
   * For methods the receiver is included in the signature, e.g.
   *   "(h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request)"
   *
   * For plain functions:
   *   "NewHandler(db *Database) *Handler"
   */
  extractFunctions(tree, sourceCode) {
    const functions = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type === 'function_declaration') {
        const fn = this._parseFuncDecl(node, sourceCode);
        if (fn) functions.push(fn);
      } else if (node.type === 'method_declaration') {
        const fn = this._parseMethodDecl(node, sourceCode);
        if (fn) functions.push(fn);
      }
    });
    return functions;
  }

  /** Parse a function_declaration node. */
  _parseFuncDecl(node, _sourceCode) {
    const nameNode = node.childForFieldName('name');
    if (!nameNode) return null;

    const params = node.childForFieldName('parameters');
    const result = node.childForFieldName('result');

    const paramsText = params ? params.text : '()';
    const resultText = result ? ' ' + result.text : '';
    const signature = `${nameNode.text}${paramsText}${resultText}`;

    return {
      name: nameNode.text,
      signature,
      startLine: node.startPosition.row + 1,
      endLine: node.endPosition.row + 1,
    };
  }

  /** Parse a method_declaration node (includes receiver). */
  _parseMethodDecl(node, _sourceCode) {
    const nameNode = node.childForFieldName('name');
    if (!nameNode) return null;

    const receiver = node.childForFieldName('receiver');
    const params = node.childForFieldName('parameters');
    const result = node.childForFieldName('result');

    const receiverText = receiver ? receiver.text + ' ' : '';
    const paramsText = params ? params.text : '()';
    const resultText = result ? ' ' + result.text : '';
    const signature = `${receiverText}${nameNode.text}${paramsText}${resultText}`;

    return {
      name: nameNode.text,
      signature,
      startLine: node.startPosition.row + 1,
      endLine: node.endPosition.row + 1,
    };
  }

  // ---------------------------------------------------------------------------
  // Imports
  // ---------------------------------------------------------------------------

  /**
   * Extract import declarations.
   *
   * Go imports come in two forms:
   *   import "fmt"
   *   import ( "fmt" ; alias "net/http" )
   *
   * Each import_spec has an optional alias (identifier, dot, or blank_identifier)
   * and a path (interpreted_string_literal).
   */
  extractImports(tree, _sourceCode) {
    const imports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'import_spec') return;

      let pathNode = null;
      let aliasNode = null;

      for (let i = 0; i < node.childCount; i++) {
        const child = node.child(i);
        if (child.type === 'interpreted_string_literal') {
          pathNode = child;
        } else if (
          child.type === 'package_identifier' ||
          child.type === 'identifier' ||
          child.type === 'dot' ||
          child.type === 'blank_identifier'
        ) {
          aliasNode = child;
        }
      }

      if (!pathNode) return;

      const source = this._stripQuotes(pathNode.text);

      // Symbol is either the explicit alias or the last path segment
      let symbol;
      if (aliasNode) {
        symbol = aliasNode.text;
      } else {
        const parts = source.split('/');
        symbol = parts[parts.length - 1];
      }

      imports.push({
        source,
        symbols: [symbol],
        isExternal: true, // Go has no relative imports
      });
    });
    return imports;
  }

  // ---------------------------------------------------------------------------
  // Exports
  // ---------------------------------------------------------------------------

  /**
   * In Go, an identifier is exported if its first character is uppercase.
   * Collect all top-level function names and type names that start with an
   * uppercase letter.
   */
  extractExports(tree, _sourceCode) {
    const exports = [];
    this._walkNodes(tree.rootNode, (node) => {
      // Top-level functions / methods
      if (node.type === 'function_declaration' || node.type === 'method_declaration') {
        const nameNode = node.childForFieldName('name');
        if (nameNode && this._isExported(nameNode.text)) {
          exports.push(nameNode.text);
        }
      }

      // Type declarations (struct, interface, type alias)
      if (node.type === 'type_spec') {
        const nameNode = node.childForFieldName('name');
        if (nameNode && this._isExported(nameNode.text)) {
          exports.push(nameNode.text);
        }
      }
    });
    return exports;
  }

  // ---------------------------------------------------------------------------
  // Classes (Go structs)
  // ---------------------------------------------------------------------------

  /**
   * Go has no classes, but struct types serve a similar role.
   * Find type_spec nodes whose type child is a struct_type.
   */
  extractClasses(tree, _sourceCode) {
    const classes = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'type_spec') return;

      const typeNode = node.childForFieldName('type');
      if (!typeNode || typeNode.type !== 'struct_type') return;

      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;

      // Use the parent type_declaration for full line span
      const declNode = node.parent && node.parent.type === 'type_declaration'
        ? node.parent
        : node;

      classes.push({
        name: nameNode.text,
        startLine: declNode.startPosition.row + 1,
        endLine: declNode.endPosition.row + 1,
      });
    });
    return classes;
  }

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

  /**
   * Extract all type_spec nodes.
   *
   * kind is determined by the type child:
   *   - struct_type   -> "struct"
   *   - interface_type -> "interface"
   *   - anything else -> "type"
   */
  extractTypes(tree, _sourceCode) {
    const types = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'type_spec') return;

      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;

      const typeNode = node.childForFieldName('type');
      let kind = 'type';
      if (typeNode) {
        if (typeNode.type === 'struct_type') kind = 'struct';
        else if (typeNode.type === 'interface_type') kind = 'interface';
      }

      // Use the parent type_declaration for full line span
      const declNode = node.parent && node.parent.type === 'type_declaration'
        ? node.parent
        : node;

      types.push({
        name: nameNode.text,
        kind,
        startLine: declNode.startPosition.row + 1,
        endLine: declNode.endPosition.row + 1,
      });
    });
    return types;
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /** Check whether a Go identifier is exported (starts with uppercase). */
  _isExported(name) {
    if (!name || name.length === 0) return false;
    const first = name.charCodeAt(0);
    return first >= 65 && first <= 90; // A-Z
  }
}
