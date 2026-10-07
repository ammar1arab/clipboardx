import { appendFileSync, cpSync, existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const library = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const dryRun = args.includes('--dry-run');
const projectArg = args.find((arg) => arg !== '--dry-run');

if (!projectArg || args.length !== (dryRun ? 2 : 1)) {
  console.error('Usage: node scripts/install.mjs <project-directory> [--dry-run]');
  process.exit(2);
}

const project = resolve(projectArg);
const destination = join(project, '.agent-library');
const entry = join(project, 'AGENTS.md');
const start = '<!-- portable-agent-library:start -->';
const block = `${start}
Follow \`.agent-library/rules/00-core.md\`. For specialized work, consult \`.agent-library/CATALOG.md\` and load only the matching rule or skill.
If \`.agent-library/PROJECT_PROFILE.md\` exists, use it only when its project facts are relevant; verify changing facts in the current tree.
Project-specific instructions and explicit user requests take precedence over generic library guidance.
<!-- portable-agent-library:end -->`;

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (!existsSync(project) || !statSync(project).isDirectory()) fail(`Project directory does not exist: ${project}`);
if (project === library || project.startsWith(`${library}${sep}`)) {
  fail('Choose a project outside the library itself.');
}
if (existsSync(destination)) fail(`Already exists; refusing to overwrite: ${destination}`);
if (existsSync(entry) && readFileSync(entry, 'utf8').includes(start)) {
  fail(`AGENTS.md already contains this library entry: ${entry}`);
}
if (existsSync(entry)) {
  const prefix = readFileSync(entry).subarray(0, 2);
  if (prefix.equals(Buffer.from([0xff, 0xfe])) || prefix.equals(Buffer.from([0xfe, 0xff]))) {
    fail(`AGENTS.md is UTF-16; add the short pointer manually to preserve its encoding: ${entry}`);
  }
}

console.log(`Copy: ${library} -> ${destination}`);
console.log(`${existsSync(entry) ? 'Append to' : 'Create'}: ${entry}`);
if (dryRun) process.exit(0);

cpSync(library, destination, { recursive: true, errorOnExist: true, force: false });
if (existsSync(entry)) {
  const previous = readFileSync(entry, 'utf8');
  const separator = previous && !previous.endsWith('\n') ? '\n\n' : previous ? '\n' : '';
  appendFileSync(entry, `${separator}${block}\n`, 'utf8');
} else {
  writeFileSync(entry, `${block}\n`, { flag: 'wx' });
}
console.log('Installed. Project customization is optional and can happen later.');
