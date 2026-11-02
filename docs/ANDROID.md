# Building for Android

The Android app is SDL2's own Android project, with the engine as its
`libmain.so`:

- **Java**: SDL's activity (`org.libsdl.app.SDLActivity`, copied from the
  SDL 2.30.9 release at build time, not kept in the repository), our
  `org.u1engine.U1Activity` (which starts it with `--game-dir <app
  files>/ULTIMA1`) and `org.u1engine.ImportActivity` (the first-run import,
  Storage Access Framework folder picker or .zip, in its own process).
- **Native**: `libSDL2.so` built from the SDL release with the NDK and
  CMake, and `crates/u1-android` (a `cdylib` named `main`), which compiles
  the desktop program's `crates/u1/src/main.rs` in, exports `SDL_main`
  and `ImportActivity.missingFiles` (the game folder check), and points
  HOME and the XDG folders into the app's private storage.
- The touch controls are the engine's: layout in
  `crates/u1-assets/src/touch.rs`, drawing and touch handling in
  `crates/u1/src/main.rs`. They are on in the Android build, or on the
  desktop with `U1_TOUCH=1` (the mouse stands in for a finger), which is
  handy for trying them.

## Tools

- JDK 17
- Android SDK: `platforms;android-34`, `build-tools;34.0.0`,
  `cmake;3.22.1`, `ndk;26.3.11579264`
- Rust with `rustup target add aarch64-linux-android x86_64-linux-android`
- `cargo install cargo-ndk`

```sh
export ANDROID_HOME=~/android-sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.3.11579264
export JAVA_HOME=~/jdk17
android/build.sh            # release; or: android/build.sh debug
```

The APK is `android/app/build/outputs/apk/release/app-release.apk`. ABIs:
`U1_ABIS` (default `arm64-v8a x86_64`; `armeabi-v7a` also works).

Signing: a release build is signed with the key in `U1_KEYSTORE`
(`U1_KEYSTORE_PASS`, `U1_KEY_ALIAS`, `U1_KEY_PASS`), else with the Android
debug key. Keep keystores out of the repository (`*.jks` and `*.keystore`
are ignored). One way to make one:

```sh
keytool -genkeypair -keystore ~/u1.jks -alias u1 -keyalg RSA -keysize 2048 -validity 10000
```

CI (`.github/workflows/android.yml`) builds it by hand or on a tag and
uploads the APK; it signs with a key made for that run.

## Trying it in the emulator

```sh
sdkmanager "emulator" "system-images;android-34;google_apis;x86_64"
avdmanager create avd -n u1test -k "system-images;android-34;google_apis;x86_64" -d pixel_6
emulator -avd u1test &
adb install android/app/build/outputs/apk/release/app-release.apk
adb push /path/to/your/ULTIMA1 /sdcard/Download/ULTIMA1
```

Then start *Ultima I* and choose `Download/ULTIMA1`. The engine's messages
go to `u1.log` in the app's files (`adb shell run-as org.u1engine.ultima1 cat files/u1.log`
works on a debug build).

## How the pieces behave

- Background: SDL pauses the program (and its sound) while the app is in
  the background and carries on when it comes back.
- Quitting: the game activity ends its process, so the next start is a
  fresh one (SDL cannot start twice in one process).
- *Import again* (an options row only on Android) leaves a `reimport`
  marker and ends the game; the activity sees it and opens the import
  screen.
- Tap actions (SPEC §24): the game hands the front end a description of
  the view at each command prompt (`u1_assets::tap::Scene`); a tap or
  click is turned into keys by `tap::resolve` and queued like any other
  key. On by default only when built for Android (`qol.tap`).
- Notches and camera holes: the window keeps out of them
  (`windowLayoutInDisplayCutoutMode=never`).
