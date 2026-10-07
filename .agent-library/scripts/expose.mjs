import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const library = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const projectArg = args[0];
const host = args[args.indexOf('--host') + 1];
const namesArg = args[args.indexOf('--skills') + 1];
const dryRun = args.includes('--dry-run');

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (!projectArg || !['shared', 'claude'].includes(host) || !namesArg) {
  fail('Usage: node scripts/expose.mjs <project> --host shared|claude --skills name1,name2 [--dry-run]');
}

const project = resolve(projectArg);
if (resolve(project, '.agent-library') !== library) {
  fail('Run the installed copy at <project>/.agent-library/scripts/expose.mjs.');
}

const names = [...new Set(namesArg.split(',').map((name) => name.trim()).filter(Boolean))];
if (!names.length) fail('Select at least one skill.');
const hostRoot = join(project, host === 'shared' ? '.agents' : '.claude', 'skills');
const operations = [];

for (const name of names) {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name)) fail(`Invalid skill name: ${name}`);
  const source = join(library, 'skills', name, 'SKILL.md');
  const target = join(hostRoot, name, 'SKILL.md');
  if (!existsSync(source)) fail(`Unknown skill: ${name}`);
  if (existsSync(join(hostRoot, name))) fail(`Target already exists; refusing to overwrite: ${target}`);
  const body = readFileSync(source, 'utf8');
  const description = body.match(/^description:\s*(.+)$/m)?.[1]?.trim();
  if (!description) fail(`Missing description in ${source}`);
  operations.push({ name, target, description });
}

for (const { name, target } of operations) console.log(`Expose ${name}: ${target}`);
if (dryRun) process.exit(0);

for (const { name, target, description } of operations) {
  mkdirSync(dirname(target), { recursive: true });
  const pointer = `---\nname: ${name}\ndescription: ${description}\n---\n\nRead and follow [the canonical skill](../../../.agent-library/skills/${name}/SKILL.md).\n`;
  writeFileSync(target, pointer, { flag: 'wx' });
}

console.log(`Exposed ${operations.length} skill(s). Check discovery in the chosen host.`);
