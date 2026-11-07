# Third-party components and notices

Project GPL-3.0-or-later licensing does not relicense the original game,
external dependencies or redistributable runtimes. No original game files
are supplied. See [CREDITS](CREDITS) for this project's art, effects and music.

## Included source / assets

- The fallback `unscii-8-subset.hex` is an unchanged subset of Viznut's
  public-domain unscii-8, not the Unifont-derived GPL variants. See
  [the font provenance](crates/u1-assets/fonts/README.md).
- Cinzel Black (Natanael Gama, SIL OFL 1.1) was used to render lettering
  in enhanced castle images. The font files themselves are not bundled;
  see [the enhanced-pack license](assets/enhanced/LICENSE).
- The parked `android-port` branch includes Gradle wrapper build tooling
  (Gradle contributors, Apache-2.0). Its wrapper configuration requests
  Gradle 8.1.1. That is separate build tooling, not original game code,
  and is not in the desktop beta packages. A copy of Apache-2.0 is in
  [LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt); see also
  [Gradle's upstream license/notices](https://github.com/gradle/gradle/blob/v8.1.1/LICENSE).
  Android is parked, not a supported beta target.

## Desktop build dependencies

Cargo.lock fixes the Rust dependency versions. Their sources and full
licenses are available from their crates.io packages; dependencies are not
all under this project's GPL license.

| Component | Declared license |
| --- | --- |
| rust-sdl2 0.37 / sdl2-sys bindings | MIT; sdl2-sys also includes SDL under zlib |
| SDL2 library | zlib |
| png 0.17, libc, bitflags, cfg-if, crc32fast, fdeflate, flate2, lazy_static | MIT or Apache-2.0 |
| adler2 | 0BSD or MIT or Apache-2.0 |
| miniz_oxide | MIT or zlib or Apache-2.0 |
| simd-adler32, version-compare | MIT |
| zlib-rs | zlib |

Upstreams: [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2),
[SDL](https://github.com/libsdl-org/SDL), and
[crates.io](https://crates.io/). Native Windows builds link SDL2 from source;
the Linux AppImage bundles SDL2 and its resolved runtime libraries, which
retain their respective licenses (not ownership of this project).

The Windows ZIP also carries Microsoft's `vcruntime140.dll` from the Visual
C++ redistributable installed on the build runner. It remains Microsoft's
runtime, not GPL project code; see
[Microsoft's redistribution terms and guidance](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files).
Windows system DLLs are not bundled. Build environments/toolchains can add
platform runtime dependencies; consult the exact package and build logs
rather than treating this table as a legal clearance certificate.
