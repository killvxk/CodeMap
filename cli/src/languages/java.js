import { LanguageAdapter } from './base.js';

/**
 * Java language adapter.
 *
 * Walks a tree-sitter AST produced by the Java grammar and extracts
 * functions (methods/constructors), imports, exports (public declarations),
 * classes and type declarations (class, interface, enum).
 */
export class JavaAdapter extends LanguageAdapter {
  constructor() {
    super('java');
  }

  // ---------------------------------------------------------------------------
  // Functions (methods + constructors)
  // ---------------------------------------------------------------------------

  extractFunctions(tree, sourceCode) {
    const functions = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (
        node.type !== 'method_declaration' &&
        node.type !== 'constructor_declaration'
      ) {
        return;
      }

      const nameNode = node.childForFieldName('name');
      if (!nameNode) return;

      // Determine the enclosing class name for qualified naming
      const className = this._findEnclosingClassName(node);
      const qualifiedName = className
        ? `${className}.${nameNode.text}`
        : nameNode.text;

      const signature = this._buildJavaSignature(node, qualifiedName, sourceCode);

      functions.push({
        name: qualifiedName,
        signature,
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
    });
    return functions;
  }

  // ---------------------------------------------------------------------------
  // Imports
  // ---------------------------------------------------------------------------

  extractImports(tree, _sourceCode) {
    const imports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (node.type !== 'import_declaration') return;

      // Get the full text of the import, e.g. "import java.util.List;"
      // or "import static java.util.Collections.sort;"
      const text = node.text.trim();

      // Remove leading "import " (and optional "static "), trailing ";"
      let path = text
        .replace(/^import\s+/, '')
        .replace(/^static\s+/, '')
        .replace(/;$/, '')
        .trim();

      // Split into source (package) and symbol
      // For "java.util.List" -> source = "java.util", symbol = "List"
      // For "java.util.*"    -> source = "java.util", symbol = "*"
      const lastDot = path.lastIndexOf('.');
      if (lastDot === -1) {
        // Single-segment import (rare, but handle it)
        imports.push({
          source: path,
          symbols: [],
          isExternal: true,
        });
        return;
      }

      const source = path.substring(0, lastDot);
      const symbol = path.substring(lastDot + 1);

      imports.push({
        source,
        symbols: [symbol],
        isExternal: true, // Java has no relative imports
      });
    });
    return imports;
  }

  // ---------------------------------------------------------------------------
  // Exports (public classes, interfaces, enums)
  // ---------------------------------------------------------------------------

  extractExports(tree, _sourceCode) {
    const exports = [];
    this._walkNodes(tree.rootNode, (node) => {
      if (
        node.type !== 'class_declaration' &&
        node.type !== 'interface_declaration' &&
        node.type !== 'enum_declaration'
      ) {
        return;
      }

      if (this._hasModifier(node, 'public')) {
        const nameNode = node.childForFieldName('name');
        if (nameNode) exports.push(nameNode.text);
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
  // Types (class, interface, enum)
  // ---------------------------------------------------------------------------

  extractTypes(tree, _sourceCode) {
    const types = [];
    const typeMap = {
      class_declaration: 'class',
      interface_declaration: 'interface',
      enum_declaration: 'enum',
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

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /**
   * Walk all nodes depth-first, calling `visitor(node)` for each.
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

  /**
   * Find the name of the enclosing class/interface/enum declaration.
   */
  _findEnclosingClassName(node) {
    let current = node.parent;
    while (current) {
      if (
        current.type === 'class_body' ||
        current.type === 'interface_body' ||
        current.type === 'enum_body'
      ) {
        const decl = current.parent;
        if (decl) {
          const nameNode = decl.childForFieldName('name');
          if (nameNode) return nameNode.text;
        }
      }
      current = current.parent;
    }
    return null;
  }

  /**
   * Check if a declaration node has a specific modifier (e.g. "public").
   */
  _hasModifier(node, modifier) {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i);
      if (child.type === 'modifiers') {
        for (let j = 0; j < child.childCount; j++) {
          if (child.child(j).text === modifier) return true;
        }
      }
    }
    return false;
  }

  /**
   * Build a human-readable signature for a Java method or constructor.
   */
  _buildJavaSignature(node, qualifiedName, _sourceCode) {
    const params = node.childForFieldName('parameters');
    const paramsText = params ? params.text : '()';

    // For methods, include return type
    if (node.type === 'method_declaration') {
      const typeNode = node.childForFieldName('type');
      const returnType = typeNode ? typeNode.text : 'void';
      return `${returnType} ${qualifiedName}${paramsText}`;
    }

    // Constructor — no return type
    return `${qualifiedName}${paramsText}`;
  }
}
