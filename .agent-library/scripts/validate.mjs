import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const required = ['README.md', 'CATALOG.md', 'SOURCES.md', 'PROJECT_PROFILE.template.md', 'rules/00-core.md'];
const failures = [];
let descriptionCharacters = 0;

for (const relative of required) {
  if (!existsSync(join(root, relative))) failures.push(`Missing ${relative}`);
}

const skillRoot = join(root, 'skills');
const skillNames = new Set();
const markdownFiles = [join(root, 'README.md'), join(root, 'CATALOG.md')];

for (const entry of readdirSync(join(root, 'rules'), { withFileTypes: true })) {
  if (entry.isFile() && entry.name.endsWith('.md')) markdownFiles.push(join(root, 'rules', entry.name));
}

for (const entry of readdirSync(skillRoot, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const file = join(skillRoot, entry.name, 'SKILL.md');
  if (!existsSync(file)) {
    failures.push(`Missing skills/${entry.name}/SKILL.md`);
    continue;
  }
  markdownFiles.push(file);
  const body = readFileSync(file, 'utf8');
  const frontmatter = body.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)?.[1];
  if (!frontmatter) {
    failures.push(`Invalid frontmatter in skills/${entry.name}/SKILL.md`);
    continue;
  }
  const name = frontmatter.match(/^name:\s*(.+)$/m)?.[1]?.trim();
  const description = frontmatter.match(/^description:\s*(.+)$/m)?.[1]?.trim();
  if (name !== entry.name) failures.push(`Name does not match folder: ${entry.name}`);
  if (!description) failures.push(`Missing description: ${entry.name}`);
  if (description?.length > 160) failures.push(`Description is too long: ${entry.name}`);
  descriptionCharacters += description?.length ?? 0;
  if (skillNames.has(name)) failures.push(`Duplicate skill name: ${name}`);
  skillNames.add(name);
}

for (const file of markdownFiles) {
  if (!existsSync(file)) continue;
  const body = readFileSync(file, 'utf8');
  if (/D:[\\/]test[\\/]|clinic-platform|volunteer-platform|medical-website|ammar-portfolio/.test(body) && !file.endsWith('README.md')) {
    failures.push(`Project-specific text in ${file.slice(root.length + 1)}`);
  }
  for (const match of body.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
    const target = match[1].split('#')[0];
    if (!target || /^[a-z]+:\/\//i.test(target)) continue;
    if (!existsSync(resolve(dirname(file), target))) {
      failures.push(`Broken link ${target} in ${file.slice(root.length + 1)}`);
    }
  }
}

if (failures.length) {
  for (const failure of failures) console.error(failure);
  process.exitCode = 1;
} else {
  const coreWords = readFileSync(join(root, 'rules/00-core.md'), 'utf8').split(/\s+/).filter(Boolean).length;
  console.log(`Valid: ${skillNames.size} skills, ${markdownFiles.length - skillNames.size - 2} rules, and internal links.`);
  console.log(`Context budget indicators: core ${coreWords} words; all skill descriptions ${descriptionCharacters} characters. Actual tokens vary by host and exposure.`);
}
