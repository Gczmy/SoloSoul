import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

/** Keep Vite's explicit optimizer complete when runtime imports are added or moved. */
export function collectDevDependencies(sourceRoot, dependencies) {
  const declared = new Set(Object.keys(dependencies));
  const imports = new Set();
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        if (!['__tests__', '__mocks__', 'test'].includes(entry.name)) visit(file);
      } else if (
        entry.isFile() &&
        /\.[cm]?[jt]sx?$/.test(entry.name) &&
        !/\.(?:test|spec|stories|d)\.[cm]?[jt]sx?$/.test(entry.name)
      ) {
        for (const { fileName } of ts.preProcessFile(readFileSync(file, 'utf8'), true, true)
          .importedFiles) {
          const packageName = fileName.startsWith('@')
            ? fileName.split('/').slice(0, 2).join('/')
            : fileName.split('/')[0];
          if (declared.has(packageName)) imports.add(fileName);
        }
      }
    }
  };
  visit(sourceRoot);
  if (declared.has('react')) {
    imports.add('react/jsx-runtime');
    imports.add('react/jsx-dev-runtime');
  }
  return [...imports].sort();
}

/** @returns {import('vite').Plugin} */
export function devDependencyUpdates(sourceRoot, dependencies, initialImports) {
  const known = new Set(initialImports);
  let restarting = false;
  return {
    name: 'solosoul-dev-dependency-updates',
    apply: 'serve',
    hotUpdate({ file, server }) {
      const relative = path.relative(sourceRoot, file);
      if (relative.startsWith('..') || path.isAbsolute(relative)) return;
      const next = collectDevDependencies(sourceRoot, dependencies);
      if (next.every((id) => known.has(id))) return;
      if (restarting) return [];
      restarting = true;
      // New dependency imports need a new explicit optimization pass, just as Vite's
      // discovery optimizer reloads after adding dependencies. Ordinary edits keep HMR.
      void server
        .restart()
        .catch((error) => {
          server.config.logger.error(`Dependency optimization restart failed: ${error.message}`);
        })
        .finally(() => {
          restarting = false;
        });
      return [];
    },
  };
}
