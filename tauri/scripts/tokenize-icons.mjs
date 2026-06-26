import * as ts from 'typescript';
import * as fs from 'fs';
import * as path from 'path';

const SRC_DIR = path.resolve(process.cwd(), 'src');

const SIZE_TO_TOKEN = {
  10: '2xs',
  11: '2xs',
  12: 'xs',
  13: 'xs',
  14: 'sm',
  16: 'md',
  18: 'lg',
  20: 'xl',
  22: '2xl',
  24: '2xl',
  28: '3xl',
  30: '3xl',
  32: '3xl',
  36: '3xl',
  40: '4xl',
  48: '5xl',
  72: '6xl',
};

const EXCLUDED_TAGS = new Set([
  'AppShell',
  'SelectCheckbox',
  'PieChartSvg',
  'ExpandableSection',
  'StatColumn',
  'Input',
  'Button',
  'Dialog',
  'Modal',
  'Avatar',
  'LoadingPlaceholder',
]);

const EXCLUDED_FILES = /\.(test|spec)\.(tsx|ts)$/;

function walkDir(dir, files = []) {
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) {
      if (ent.name === 'node_modules' || ent.name === 'dist' || ent.name === '.vite') continue;
      walkDir(p, files);
    } else if (ent.name.endsWith('.tsx') && !EXCLUDED_FILES.test(ent.name)) {
      files.push(p);
    }
  }
  return files;
}

function getTagText(tagName) {
  if (ts.isIdentifier(tagName)) return tagName.text;
  if (ts.isQualifiedName(tagName)) return getTagText(tagName.left) + '.' + tagName.right.text;
  if (tagName.kind === ts.SyntaxKind.JsxMemberExpression) {
    const member = tagName;
    return getTagText(member.object) + '.' + member.name.text;
  }
  if (tagName.kind === ts.SyntaxKind.JsxNamespacedName) {
    const ns = tagName;
    return ns.namespace.text + ':' + ns.name.text;
  }
  return '';
}

function isExcludedTag(tagNameNode) {
  const text = getTagText(tagNameNode);
  if (!text) return true;
  if (text[0] !== text[0].toUpperCase()) return true; // HTML-like lowercase
  return EXCLUDED_TAGS.has(text);
}

function tokenName(n) {
  return SIZE_TO_TOKEN[n];
}

function processFile(filePath) {
  const sourceText = fs.readFileSync(filePath, 'utf8');
  const sourceFile = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );

  const replacements = [];
  let hasIconSize = false;

  function visit(node) {
    let attrs = null;
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      attrs = node.attributes;
    }
    if (attrs && !isExcludedTag(node.tagName)) {
      for (const attr of attrs.properties) {
        if (
          ts.isJsxAttribute(attr) &&
          (attr.name.text === 'size' || attr.name.text === 'iconSize') &&
          attr.initializer &&
          ts.isJsxExpression(attr.initializer) &&
          attr.initializer.expression &&
          ts.isNumericLiteral(attr.initializer.expression)
        ) {
          const n = Number(attr.initializer.expression.text);
          const token = tokenName(n);
          if (token) {
            const expr = attr.initializer;
            const access = /^\d/.test(token) ? `['${token}']` : `.${token}`;
            replacements.push({
              start: expr.getStart(sourceFile),
              end: expr.getEnd(),
              text: `{ICON_SIZE${access}}`,
            });
            if (attr.name.text === 'iconSize') hasIconSize = true;
          } else {
            console.warn(`[skip] ${path.relative(process.cwd(), filePath)}: unsupported size ${n} on <${getTagText(node.tagName)}>`);
          }
        }
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);

  if (replacements.length === 0) return false;

  // Check if ICON_SIZE is already imported
  let hasImport = false;
  let lastImportEnd = 0;
  ts.forEachChild(sourceFile, (node) => {
    if (ts.isImportDeclaration(node)) {
      lastImportEnd = node.getEnd();
      const moduleSpecifier = node.moduleSpecifier.getText(sourceFile).replace(/['"]/g, '');
      if (moduleSpecifier === '@/lib/iconSizes') {
        const named = node.importClause?.namedBindings;
        if (named && ts.isNamedImports(named)) {
          for (const el of named.elements) {
            if (el.name.text === 'ICON_SIZE') hasImport = true;
          }
        }
      }
    }
  });

  replacements.sort((a, b) => b.start - a.start);
  let newText = sourceText;
  for (const r of replacements) {
    newText = newText.slice(0, r.start) + r.text + newText.slice(r.end);
  }

  if (!hasImport) {
    const importLine = `import { ICON_SIZE } from '@/lib/iconSizes';\n`;
    if (lastImportEnd > 0) {
      newText = newText.slice(0, lastImportEnd) + '\n' + importLine + newText.slice(lastImportEnd);
    } else {
      newText = importLine + newText;
    }
  }

  fs.writeFileSync(filePath, newText, 'utf8');
  return true;
}

const files = walkDir(SRC_DIR);
let changed = 0;
for (const file of files) {
  if (processFile(file)) {
    console.log(`[changed] ${path.relative(process.cwd(), file)}`);
    changed++;
  }
}
console.log(`\nDone: ${changed} files changed.`);
