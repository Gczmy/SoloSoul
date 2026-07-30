const { Project, SyntaxKind } = require('ts-morph');
const fs = require('fs');
const path = require('path');

// ---------------------------------------------------------------------------
// 1. Parse Rust commands and build a map of command -> [param names]
// ---------------------------------------------------------------------------

const COMMANDS = {};

function findMatchingParen(text, openIdx) {
  let depth = 0;
  for (let i = openIdx + 1; i < text.length; i++) {
    const c = text[i];
    if (c === '(' || c === '[' || c === '<') depth++;
    else if (c === ')' || c === ']' || c === '>') {
      if (depth === 0) return i;
      depth--;
    }
  }
  return -1;
}

function parseRustCommandParams(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const commandRe = /#\[tauri::command\]\s*(?:#\[\w+(?:\([^)]*\))?\]\s*)*\s*(?:pub\s+async\s+fn|pub\s+fn)\s+(\w+)\s*\(/g;
  let match;
  while ((match = commandRe.exec(content)) !== null) {
    const funcName = match[1];
    const openIdx = content.indexOf('(', match.index + match[0].length - 1);
    if (openIdx === -1) continue;
    const closeIdx = findMatchingParen(content, openIdx);
    if (closeIdx === -1) continue;
    const paramsText = content.slice(openIdx + 1, closeIdx);
    const params = [];
    let depth = 0;
    let current = '';
    for (const ch of paramsText) {
      if (ch === '<' || ch === '(' || ch === '[') depth++;
      if (ch === '>' || ch === ')' || ch === ']') depth--;
      if (ch === ',' && depth === 0) {
        params.push(current.trim());
        current = '';
      } else {
        current += ch;
      }
    }
    if (current.trim()) params.push(current.trim());

    const paramNames = [];
    for (const p of params) {
      const m = p.match(/^(\w+)\s*:\s*(.+)$/);
      if (!m) continue;
      const [, pname, ptype] = m;
      const typeNorm = ptype.replace(/\s+/g, ' ').trim();
      if (
        typeNorm.startsWith("State<") ||
        typeNorm.startsWith("tauri::State<") ||
        typeNorm.startsWith("AppHandle") ||
        typeNorm.startsWith("Window") ||
        typeNorm.startsWith("WebviewWindow") ||
        typeNorm.startsWith("Webview") ||
        typeNorm.startsWith("Channel") ||
        typeNorm.startsWith("Event") ||
        typeNorm.startsWith("tauri::AppHandle")
      ) {
        continue;
      }
      paramNames.push(pname);
    }
    COMMANDS[funcName] = paramNames;
  }
}

const rustFiles = [];
function walkRust(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) walkRust(p);
    else if (entry.name.endsWith('.rs')) rustFiles.push(p);
  }
}
walkRust(path.join(__dirname, '../src-tauri/src'));
rustFiles.forEach(parseRustCommandParams);

// ---------------------------------------------------------------------------
// 2. TypeScript transformation: snake_case -> camelCase for invoke payloads
// ---------------------------------------------------------------------------

function snakeToCamel(str) {
  return str.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase());
}

function camelToSnake(str) {
  return str.replace(/[A-Z]/g, (ch) => '_' + ch.toLowerCase()).replace(/^_/, '');
}

const tsConfigs = [
  { name: 'tauri', filePath: path.join(__dirname, '../tsconfig.json') },
  { name: 'sdk-js', filePath: path.join(__dirname, '../../sdk/js/tsconfig.json') },
];

const allSourceFiles = [];
for (const tc of tsConfigs) {
  if (!fs.existsSync(tc.filePath)) {
    console.warn(`Skipping ${tc.name}: ${tc.filePath} not found`);
    continue;
  }
  const proj = new Project({ tsConfigFilePath: tc.filePath });
  allSourceFiles.push(...proj.getSourceFiles().map((sf) => ({ sourceFile: sf, projectName: tc.name })));
}

let totalRenamed = 0;
let totalFiles = 0;
const notMatched = [];

for (const { sourceFile } of allSourceFiles) {
  const filePath = sourceFile.getFilePath();
  if (!/\.(ts|tsx)$/.test(filePath)) continue;
  if (filePath.includes('node_modules')) continue;
  if (!filePath.includes('/src/') && !filePath.includes('\\src\\')) continue;

  const changes = [];

  function processObject(objArg, expectedParams) {
    const props = objArg.getProperties();
    const expectedSnake = new Set(expectedParams);

    for (const prop of props) {
      const kind = prop.getKind();
      let name;
      if (kind === SyntaxKind.PropertyAssignment || kind === SyntaxKind.ShorthandPropertyAssignment) {
        name = prop.getName();
      } else {
        continue;
      }

      if (!expectedSnake.has(name)) continue;
      const camel = snakeToCamel(name);
      if (camel === name) continue;

      if (kind === SyntaxKind.ShorthandPropertyAssignment) {
        // Should not happen after forward codemod, but keep safe
        changes.push({
          start: prop.getStart(),
          end: prop.getEnd(),
          replacement: `${camel}: ${name}`,
        });
        totalRenamed++;
      } else {
        const valueNode = prop.getInitializer();
        const valueText = valueNode ? valueNode.getText() : name;
        if (valueText === name && valueText !== camel) {
          // e.g. { account_id: accountId } -> use shorthand { accountId }
          changes.push({
            start: prop.getStart(),
            end: prop.getEnd(),
            replacement: camel,
          });
        } else {
          changes.push({
            start: prop.getNameNode().getStart(),
            end: prop.getNameNode().getEnd(),
            replacement: camel,
          });
        }
        totalRenamed++;
      }
    }
  }

  for (const call of sourceFile.getDescendantsOfKind(SyntaxKind.CallExpression)) {
    const expr = call.getExpression();
    const fnName = expr.getKind() === SyntaxKind.PropertyAccessExpression ? expr.getName() : expr.getText();
    const args = call.getArguments();

    let cmdName = null;
    let objArg = null;

    if (fnName === 'invoke') {
      if (args[0] && args[0].getKind() === SyntaxKind.StringLiteral) {
        cmdName = args[0].getText().slice(1, -1);
      }
      objArg = args[1];
    } else if (fnName === 'toHaveBeenCalledWith' || fnName === 'toHaveBeenLastCalledWith') {
      if (args[0] && args[0].getKind() === SyntaxKind.StringLiteral) {
        cmdName = args[0].getText().slice(1, -1);
      }
      objArg = args[1];
    } else if (fnName === 'toHaveBeenNthCalledWith') {
      if (args[1] && args[1].getKind() === SyntaxKind.StringLiteral) {
        cmdName = args[1].getText().slice(1, -1);
      }
      objArg = args[2];
    } else if (
      fnName === 'objectContaining' ||
      (expr.getKind() === SyntaxKind.PropertyAccessExpression && fnName.endsWith('.objectContaining'))
    ) {
      let ancestor = call.getParent();
      while (ancestor) {
        if (ancestor.getKind && ancestor.getKind() === SyntaxKind.CallExpression) {
          const aExpr = ancestor.getExpression && ancestor.getExpression();
          const aName = aExpr ? aExpr.getText() : '';
          if (aName === 'toHaveBeenCalledWith' || aName === 'toHaveBeenLastCalledWith') {
            const aArgs = ancestor.getArguments();
            if (aArgs[0] && aArgs[0].getKind() === SyntaxKind.StringLiteral) {
              cmdName = aArgs[0].getText().slice(1, -1);
              break;
            }
          } else if (aName === 'toHaveBeenNthCalledWith') {
            const aArgs = ancestor.getArguments();
            if (aArgs[1] && aArgs[1].getKind() === SyntaxKind.StringLiteral) {
              cmdName = aArgs[1].getText().slice(1, -1);
              break;
            }
          }
        }
        ancestor = ancestor.getParent();
      }
      objArg = args[0];
    }

    if (!cmdName || !COMMANDS[cmdName]) continue;
    if (!objArg) continue;

    if (objArg.getKind() === SyntaxKind.ObjectLiteralExpression) {
      processObject(objArg, COMMANDS[cmdName]);
    }
  }

  if (changes.length === 0) continue;

  changes.sort((a, b) => b.start - a.start);
  for (const change of changes) {
    sourceFile.replaceText([change.start, change.end], change.replacement);
  }
  sourceFile.saveSync();
  totalFiles++;
}

console.log(`Reverted ${totalRenamed} keys across ${totalFiles} files.`);
if (notMatched.length > 0) {
  console.log(`\nPossibly stale snake_case keys not matching any command param (${notMatched.length}):`);
  for (const m of notMatched.slice(0, 50)) {
    console.log(`- ${m}`);
  }
}
