---
name: module-scaffold
description: Start a new module or package using the project's actual registration, routing, naming, and test conventions. Use for scaffolding requests.
---

# Module scaffold

Inspect the current module registry, a similar module, and the project's naming and validation commands. Identify which files are required to make the requested module load or build. Do not assume a fixed folder tree, framework, route file, or permission model.

Create only the files needed for the requested readiness level. Avoid empty directories and invented API contracts. Register the module where the project expects it; do not leave an active route or export pointing at a missing file.

Check imports, registration, and the smallest relevant build or validator. Report which paths are usable now and which requested behavior is still deferred. Mark real deferred work explicitly without presenting a stub as a complete feature.
