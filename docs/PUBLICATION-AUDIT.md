# Pre-publication review (2026-11-07)

This is a practical repository review, not legal advice, a provenance
certificate, or a guarantee that every possible secret/copyright issue can
be detected. No history was rewritten and no branches/tags were removed.

## Scope

Before changing visibility, reviewed all seven remote branch tips
(`android-port`, `ci-appimage`, `features-review-1`, `main`, `polish-1`,
`space-aim-alt`, `windows-portable`) and both published version tags
(`v0.1.0-alpha.1`, `v0.1.0-beta.1`), including their reachable history:
185 commits and 1,091 unique blobs before this documentation cleanup.

- Inventoried every historical path and binary type, including the parked
  Android branch. No original EXE/BIN/U1/COM/CASTLE data file paths or
  executable payloads were found in Git objects.
- Compared blob SHA-256 hashes against the owned reference game-file set:
  no exact matches. Hash comparison cannot detect every transformed asset.
- Pattern-scanned historical blobs for common credential formats, private
  keys, credential-bearing URLs and decompiler markers; reviewed author
  metadata and contact/private-host literals. No credentials or private
  personal details were identified. This was not a dedicated forensic or
  exhaustive entropy-based secret audit.
- Compared substantive consecutive source/doc lines with reference-study
  assembly/text files. The only matches were an old project-generated 5x7
  fallback glyph set, explicitly documented as synthetic and later replaced
  by the attributed public-domain unscii fallback. No original disassembly
  or decompiled executable listings were identified in the repository.
- Visually reviewed contact sheets of all 194 distinct historical PNGs:
  enhanced project art, synthetic demo/icons and their revisions. No
  obvious original-game screenshot or extracted original artwork was
  identified. Visual review does not independently prove authorship.
- Reviewed the project GPL license, enhanced-pack CC-BY-SA attribution,
  font provenance, Cargo dependency license metadata and parked Gradle
  wrapper tooling. Added clearer third-party notices; those components
  retain their own licenses, not the project's blanket GPL claim.
- Inspected the existing alpha AppImage and both beta payloads for original
  game-like files. None were found; the Windows engine exe and Microsoft
  runtime DLL are expected engine/runtime content, not original game EXEs.
  Beta source/version provenance was verified during promotion.
- No issues or pull requests existed at review time. This review does not
  claim to have inspected every old Actions log/artifact, or content outside
  published repository refs and the release assets described above.

## Publication boundaries

Original game and reference-study files remain outside the repository.
Read game tables/text from the user's copy; do not commit extracted data,
listings, ROM fonts or original-art screenshots. Licensed dependencies and
build tooling are separate (see [THIRD_PARTY.md](../THIRD_PARTY.md)).

The code was developed from reference study and a written behavior spec;
it is **not** claimed to be a strict clean-room project. Short behavioral
examples in specs/tests, trademarked names and independently drawn art do
not constitute a legal clearance determination. Report a concern privately
to the repository owner before posting suspected secret values or game data.

The existing beta tag/assets remain unchanged by this documentation cleanup.
macOS is not a beta target and Android remains parked, not merged into main.
