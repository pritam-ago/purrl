# 🛣️ purrl Roadmap

What is built is in the README. This is what is not.

## v0.1 (current)

- [x] Semantic search over file names
- [x] Open files from results
- [x] Index progress bar
- [x] ASCII cat banner
- [x] Index stored outside the working directory
- [x] Tests and CI

## v0.2

- [ ] Filename-match ranking term
- [ ] Selection-history learning — rank what you actually pick higher
- [ ] Recency-based ranking boost
- [ ] Debug scoring mode
- [ ] Better intent parsing
- [ ] Configurable index roots

The first two were listed as done in v0.1 until this was corrected. They were never
written: ranking is cosine similarity alone.

## v0.3

- [ ] PDF text extraction
- [ ] Image EXIF metadata
- [ ] Incremental indexing
- [ ] File preview (`read`) — needs extraction first
- [ ] Summarization (`summarize`) — needs extraction first

`read` and `summarize` existed as stubs that printed "not implemented yet". They were
removed, because both need content extraction that does not exist yet.

## v1.0

- [ ] Stable release
- [ ] Plugin system
- [ ] Background indexing (optional)

## Not planned

- **Deleting files.** purrl finds things. Removing them is not its job, and a
  destructive action driven by a language model's intent parsing is a bad idea in any
  version.
