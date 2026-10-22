# Packaged Fontdone local search spot check

Date: 22 October 2026. Source commit: `d5c5c630cfd67bb8fc1a4fc1068b3a3ab20978c7`.
The installed `Code Atlas.app` binary matched the release bundle at SHA-256
`e0d909f2c441f19e389d32114a7519ac64ac85c14f6964017d02bc8cdce49bf9`.

## Scope

The installed app was opened on the saved Fontdone snapshot
`main@5249b723719445813b8771a602ea5b96891d4450`. Search was performed from
the repository sidebar. The Codex connection was present, but no question,
Guide, or model request was started for this search check.

## Observations

- Searching `memory_face` returned `src/font.rs:4053` first, with additional
  results in the large FFI and C ABI implementation files.
- Opening that result displayed the captured `Font::memory_face` implementation
  from lines 4053–4115, including the PFR0 dispatch and nearby format branches.
- Searching `font bytes rendered glyph bitmap entry parsing outline hinting
  rasterization` displayed 32 results and more matches. The first page contained
  broad module, fixture, and adapter hits; it did not surface the
  `Font::memory_face` declaration. This agrees with the weak first-page recall
  in the frozen native retrieval profile.

## Conclusion

The packaged search UI can locate and open a declaration inside the 375,700-byte
`src/font.rs`, and it exposes results from the large adapter files without
asking the user to attach them manually. Broad global search is still too noisy
to establish that the relevant end-to-end path will be found automatically.
This spot check does not qualify exact range integrity across all result types,
parserless files over 1 MiB, cancellation, index corruption/rebuild, or an
agent-generated answer. Keep Kata `a699` and its parent `0wdn` open.
