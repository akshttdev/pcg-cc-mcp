# How to Submit Tauri Issue/Documentation PR

## Quick Links

**Tauri Repository:** https://github.com/tauri-apps/tauri
**Issue Template:** `/tmp/tauri-pr-description.md`
**Tauri Docs:** https://github.com/tauri-apps/tauri-docs

## Option A: File an Issue (Recommended)

This documents the problem and links to the zerocopy fix.

### 1. Go to Tauri Issues

https://github.com/tauri-apps/tauri/issues/new/choose

### 2. Select "Bug Report"

### 3. Fill Out Template

**Title:**
```
Compilation fails on stable Rust due to zerocopy AVX-512 dependency
```

**Description:**
Copy from `/tmp/tauri-pr-description.md` and add:

```markdown
## Current Status

A fix has been submitted to zerocopy: [link to your PR]

Once merged, Tauri builds will work on stable Rust without workarounds.

## Temporary Workaround

Users can add this to their `Cargo.toml`:

\`\`\`toml
[patch.crates-io]
zerocopy = "=0.7.35"
\`\`\`

## Affected Versions

- Tauri 2.10.x
- Rust stable 1.89-1.90
- zerocopy 0.8.23+

## Related Issues

- google/zerocopy#XXXX (PR with fix)
- rust-lang/rust#111137 (AVX-512 tracking)
```

### 4. Add Labels

- `type: bug`
- `OS: linux`
- `status: needs triage`

### 5. Submit

Click **"Submit new issue"**

## Option B: Documentation PR (Advanced)

If you want to add the workaround to Tauri docs:

### 1. Fork tauri-docs

```bash
git clone https://github.com/YOUR-USERNAME/tauri-docs.git
cd tauri-docs
```

### 2. Add Troubleshooting Section

Edit: `docs/guides/building/linux.md`

Add section:
```markdown
### zerocopy AVX-512 Compilation Error (Rust 1.89-1.90)

If you encounter this error:
\`\`\`
error[E0658]: use of unstable library feature 'stdarch_x86_avx512'
\`\`\`

Add this workaround to your `Cargo.toml`:

\`\`\`toml
[patch.crates-io]
zerocopy = "=0.7.35"
\`\`\`

This will be resolved in a future Tauri version once zerocopy 0.8.40+ is available.

See: [zerocopy issue #XXXX](link)
```

### 3. Submit PR

```bash
git checkout -b docs/zerocopy-workaround
git add docs/guides/building/linux.md
git commit -m "docs: Add zerocopy AVX-512 workaround"
git push origin docs/zerocopy-workaround
```

Create PR at: https://github.com/tauri-apps/tauri-docs/pulls

## Direct Links

**File Issue:** https://github.com/tauri-apps/tauri/issues/new
**Existing Issues:** https://github.com/tauri-apps/tauri/issues?q=is%3Aissue+zerocopy

---

**Note:** The Tauri team will likely wait for the zerocopy fix to merge before making changes. Filing an issue helps track the problem and informs other users.
