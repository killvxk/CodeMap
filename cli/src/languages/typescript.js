import { LanguageAdapter } from './base.js';

/**
 * TypeScript / JavaScript language adapter.
 *
 * Walks a tree-sitter AST produced by the TypeScript grammar and extracts
 * functions, imports, exports, classes and type declarations.
 *
 * Also used for plain JavaScript files (the TypeScript grammar is a superset).
 */
export class TypeScriptAdapter extends LanguageAdapter {
  constructor() {
    super('typescript');
  }

  // ---------------------------------------------------------------------------
  // Functions
  // ---------------------------------------------------------------------------

  /**
   * Collect all top-level function declarations (including those wrapped in
   * export_statement) as well as arrow-function variable declarations that are
   * exported.
   */
  extractFunctions(tree, sourceCode) {
    const functions = [];
    this._walkNodes(tree.rootNode, (node) => {
      // Direct function_declaration (top-level or inside export_statement)
      if (node.type === 'function_declaration') {
        const fn = this._parseFunctionDeclaration(node, sourceCode);
        if (fn) functions.push(fn);
        return; // no need to recurse into the body
      }

      // Arrow functions assigned to const/let at top level
      // e.g. export const foo = (args) => { ... }
      if (
        node.type === 'lexical_declaration' &&
        node.parent &&
        (node.parent.type === 'program' || node.parent.type === 'export_statement')
      ) {
        for (let i = 0; i < node.namedChildCount; i++) {
          const declarator = node.namedChild(i);
          if (declarator.type !== 'variable_declarator') continue;
          const value = declarator.childForFieldName('value');
          if (value && value.type === 'arrow_function') {
            const nameNode = declarator.childForFieldName('name');
            if (!nameNode) continue;
            const params = value.childForFieldName('parameters');
            const retType = value.childForFieldName('return_type');
            const signature = this._buildSignature(nameNode.text, params, retType, sourceCode);
            functions.push({
              name: nameNode.text,
              signature,
              startLine: node.startPosition.row + 1,
              endLine: node.endPosition.row + 1,
            });
          }
        }
      }
    });
    return functions;
  }

  /** Parse a function_declaration node into a structured record. */
  _parseFunctionDeclaration(node, sourceCode) {
    const nameNode = node.childForFieldName('name');
    if (!nameNode) return null;
    const params = node.childForFieldName('parameters');
    const retType = node.childForFieldName('return_type');
    const signature = this._buildSignature(nameNode.text, params, retType, sourceCode);
    return {
      name: nameNode.text,
      signature,
      startLine: node.startPosition.row + 1,
      endLine: node.endPosition.row + 1,
    };
  }

  /** Build a human-readable signature string from name + params + return type. */
  _buildSignature(name, paramsNode, returnTypeNode, _sourceCode) {
    const params = paramsNode ? paramsNode.text : '()';
    const ret = returnTypeNode ? returnTypeNode.text : '';
    return `${name}${params}${ret}`;
  }

  // ---------------------------------------------------------------------------
  // Imports
  // ---------------------------------------------------------------------------

  extractImports(tree, _sourceCode) {
    const imports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'import_statement') return;

      const sourceNode = node.childForFieldName('source')
        || this._findChildOfType(node, 'string');
      if (!sourceNode) return;

      const raw = this._stripQuotes(sourceNode.text);
      const symbols = [];

      // Named imports  { a, b }
      const importClause = this._findChildOfType(node, 'import_clause');
      if (importClause) {
        const namedImports = this._findChildOfType(importClause, 'named_imports');
        if (namedImports) {
          for (let i = 0; i < namedImports.namedChildCount; i++) {
            const spec = namedImports.namedChild(i);
            if (spec.type === 'import_specifier') {
              const nameNode = spec.childForFieldName('name') || spec.firstNamedChild;
              if (nameNode) symbols.push(nameNode.text);
            }
          }
        }
        // Default import  import x from '...'
        for (let i = 0; i < importClause.namedChildCount; i++) {
          const child = importClause.namedChild(i);
          if (child.type === 'identifier') {
            symbols.push(child.text);
          }
        }
      }

      imports.push({
        source: raw,
        symbols,
        isExternal: !raw.startsWith('.'),
      });
    });
    return imports;
  }

  // ---------------------------------------------------------------------------
  // Exports
  // ---------------------------------------------------------------------------

  extractExports(tree, _sourceCode) {
    const exports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'export_statement') return;

      // export function foo / export async function foo
      const funcDecl = this._findChildOfType(node, 'function_declaration');
      if (funcDecl) {
        const nameNode = funcDecl.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }

      // export class Foo
      const classDecl = this._findChildOfType(node, 'class_declaration');
      if (classDecl) {
        const nameNode = classDecl.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }

      // export interface Foo
      const ifaceDecl = this._findChildOfType(node, 'interface_declaration');
      if (ifaceDecl) {
        const nameNode = ifaceDecl.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }

      // export type Foo = ...
      const typeAlias = this._findChildOfType(node, 'type_alias_declaration');
      if (typeAlias) {
        const nameNode = typeAlias.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
      }

      // export const/let/var
      const lexDecl = this._findChildOfType(node, 'lexical_declaration');
      if (lexDecl) {
        for (let i = 0; i < lexDecl.namedChildCount; i++) {
          const decl = lexDecl.namedChild(i);
          if (decl.type === 'variable_declarator') {
            const nameNode = decl.childForFieldName('name');
            if (nameNode) exports.push(nameNode.text);
          }
        }
      }

      // export { a, b, c }
      const exportClause = this._findChildOfType(node, 'export_clause');
      if (exportClause) {
        for (let i = 0; i < exportClause.namedChildCount; i++) {
          const spec = exportClause.namedChild(i);
          if (spec.type === 'export_specifier') {
            const nameNode = spec.childForFieldName('name') || spec.firstNamedChild;
            if (nameNode) exports.push(nameNode.text);
          }
        }
      }
    });
    return exports;
  }

  // ---------------------------------------------------------------------------
  // Classes
  // ---------------------------------------------------------------------------

  extractClasses(tree, _sourceCode) {
    const classes = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'class_declaration') return;
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
  // Types (interfaces + type aliases)
  // ---------------------------------------------------------------------------

  extractTypes(tree, _sourceCode) {
    const types = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type === 'interface_declaration') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
          types.push({
            name: nameNode.text,
            kind: 'interface',
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      } else if (node.type === 'type_alias_declaration') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
          types.push({
            name: nameNode.text,
            kind: 'type',
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
