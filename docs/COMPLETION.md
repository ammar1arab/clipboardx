# ClipboardX completion ledger

The project profile and the user's full twelve-milestone completion specification define
the scope below. The specification was supplied on 2026-10-07.
This ledger records requirements, not a claim of production readiness.

## Implementation sequence

1. Rust persistence and validated native contracts: text and image metadata in SQLite,
   image files under app-data, stable IDs, deduplication, pinning, editing, deletion,
   and configurable retention. Add regression coverage at the persistence boundary.
2. Native clipboard monitoring: Wayland text and images, thumbnails, ownership lifetime,
   bounded resource use, capture pause, explicit error states, and shutdown behavior.
3. React history interface: coherent tokens, search, focused selection, keyboard navigation,
   editing, multi-selection, thumbnails, loading, empty and failure states.
4. COSMIC integration: Super+V launch, focus restoration, immediate paste for click/Enter,
   combined paste for Ctrl+Enter, autostart, hidden-window lifecycle, native appearance.
   Verify the actual compositor-supported mechanisms before choosing an implementation.
5. Release validation: automated checks, packaged build, clean-machine installation,
   privacy review, measured startup, and the manual acceptance procedure below.

## Current evidence

- Milestone 1 has a graphite UI shell with typed sample data, local preview actions,
  search, arrow navigation, inline editing, pinning, multi-selection and error states.
  Native paste is visibly unavailable in this preview; it is not a working paste feature.
  The user's subsequent cleanup request replaces the preview with real persisted history;
  sample entries and preview mutations have been removed from application code.
- Eight React Testing Library/user-event interaction tests pass. Frontend build and
  Rust fmt/check/Clippy (warnings denied)/tests pass on 2026-10-07.
- Vite runs at `127.0.0.1:1420`. Computer control reports no browser/native app surfaces;
  opening the in-app browser fails with "Browser is not available: iab". Visual review
  and desktop acceptance have not been performed.
- Rust now owns a text-history repository and `list_history` / `capture_text` commands.
- Text is stored in an app-data SQLite database; on Unix its parent directory is mode 0700.
- Regression tests cover exact text preservation, stable duplicate IDs across reopening,
   and rejected empty, oversized, or NUL-containing payloads.
- Milestone 2 provides versioned transactional migrations, a shared text/image metadata
  schema, Rust-owned text capture/list/edit/delete/pin/clear operations and persistent
  count/age retention. Pins survive retention; edits into existing text are rejected
  without mutating either entry. Clear/delete remain idempotent where appropriate.
- Nine Rust regression tests pass, including legacy upgrade, repeated migration, rapid
  recopy promotion, invalid operation atomicity, settings/pin persistence and cleanup.
  Native commands return `{ code, message }` errors without database/clipboard details.
  Image insertion and filesystem cleanup will be verified in milestone 4.
- Milestone 3 implements event-driven Wayland text capture on a dedicated Rust worker,
  bounded 1 MiB/2-second offer reads, UTF-8 validation, exclusion-hint handling,
  automatic reconnection, status/retry commands and cancellation on app exit.
- Thirteen deterministic Rust tests pass; one live-desktop test is excluded from ordinary
  CI and was run explicitly with `cargo test --offline live_wayland_text_events_and_cancellation
  -- --ignored --test-threads=1`: passed on the current COSMIC Wayland session. It verifies
  short English, mixed Arabic/English multiline text, URL offers and watcher cancellation.
  This verifies native event transfer, not copying from named applications or popup paste.
- Images, native paste, launcher, settings,
  autostart state, release packaging and hardening remain outstanding.

## Current cleanup and live integration

The window now reads real SQLite history through native commands and listens for capture
events, with debounced refreshes, structured error display and retry. Edit, deletion and
pinning use Rust persistence. Native paste remains explicitly unavailable until milestone 7.
Eleven frontend tests verify the shell and live command/event integration. This connection
was brought forward at the user's request to remove dummy text.

Popup dimensions are configured at 460 × 520, minimum 360 × 360. Source comments, starter
logos and greeting code are removed. Unused SQL/store/opener plugins, their permissions and
six frontend dependencies are removed. One capability file grants event subscriptions and
official autostart operations. The remaining separate modules own meaningful concerns.

Tauri development startup was exercised, and its process handle remained live until stopped
for rebuild. Frontend hot reload produced IPC fallback/callback warnings; clean-start
behavior still needs verification. Window size and visuals are not claimed as observed.

## Required automated gates

Run against the final tree and record results:

```sh
npm run build
npm test
cd src-tauri
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
```

Also verify a packaged Tauri build and the release artifact's launch on the target system.
Tests must cover actual selection behavior, editing, deletion, pinning, retention,
deduplication, text/image persistence and native boundary failures.

## Manual acceptance on Pop!_OS 24.04 COSMIC Wayland

Record OS/compositor versions, installation artifact, test date and observed results.
Use synthetic clipboard fixtures only. Each unchecked criterion remains unverified.

Run copy/paste against Chrome, Cursor, terminal, ordinary text inputs and multiline
inputs. Copy short/long text, Arabic, English, mixed Arabic/English, URLs, source code,
multiline text, duplicates and screenshots. Repeat rapid copy/paste, restart, empty
database and large-history cases. Test all listed shortcuts, including Ctrl+P and Escape.

- [ ] Install and launch the packaged app; measure cold/warm startup and agree a concrete
      performance threshold with the completion specification.
- [ ] Copy text in two native applications; each appears once, with exact whitespace.
      Recopying promotes the same entry without losing its pin or creating a duplicate.
- [ ] Copy screenshots and application images; correct thumbnails appear and remain after
      restart. Inspect storage to confirm image files live under app-data.
- [ ] Search text; clear the search; navigate with arrows; selection remains predictable.
- [ ] Press Super+V from a native editor and a browser; the launcher focuses reliably.
- [ ] Click a text/image entry or press Enter; launcher closes, original app regains focus,
      and exactly one paste occurs at its caret. Repeat with more than one application.
- [ ] Ctrl+Click selects independent entries; Shift+Click selects a contiguous range;
      Ctrl+A selects all relevant entries; Ctrl+Enter pastes the selected items in the
      documented order. Confirm the specified mixed-image/text behavior.
- [ ] F2 edits text; Enter saves; Escape cancels; verify duplicate-edit behavior.
- [ ] Delete removes selected entries and associated unused image files.
- [ ] Pin/unpin survives restart and pinned items are protected from retention.
- [ ] Change retention; verify its documented count/age policy on synthetic data.
- [ ] Clear unpinned preserves pins; clear history removes all entries after confirmation.
- [ ] Settings persist history limit, image capture toggle and paste separator; global
      shortcut is configurable wherever technically supported.
- [ ] Enable autostart, log out/in, verify background capture; disable and verify no start.
- [ ] Closing/dismissing the launcher does not stop capture or leave a focused invisible
      window. Repeated launches do not create duplicate processes or monitors.
- [ ] Test unavailable clipboard/paste support and storage failures; actionable errors
      appear without clipboard contents in logs.
- [ ] Inspect focus visibility, compact layout, scrolling, image loading, display scaling,
      and keyboard-only operation. Record screenshots for visual review.
- [ ] Confirm no clipboard data is sent to network services and no content is logged.

## Completion rule

Every project-profile feature and every item in the supplied completion specification
must map to implementation and current verification evidence. Passing repository tests
alone cannot establish COSMIC focus, global launcher, or paste acceptance.

Final acceptance also requires migrations, structured native errors, minimal capabilities,
app-data-only validated image access, orphan cleanup, removal of dead starter code and
dependencies, a task/thread lifecycle review, measured performance, a running Tauri
development app, a successful production package and a clean working tree. Each green
milestone receives a focused commit. The final report must include architecture, every
feature's pass/fail evidence, automated/manual results, unverified manual cases, known
limitations, build/install/usage instructions, artifact paths and commit IDs.
