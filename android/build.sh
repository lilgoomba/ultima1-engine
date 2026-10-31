#!/usr/bin/env bash
# Builds the Android APK: SDL2 (from its release tarball) and the engine
# (crates/u1-android, as libmain.so) for each ABI, then the Java side with
# Gradle. See PLAYING-ANDROID.md and docs/ANDROID.md.
#
# Needs: ANDROID_HOME (platform 34, build-tools 34.0.0, cmake 3.22.1),
# ANDROID_NDK_HOME (r26), JDK 17, rustup targets for the ABIs, cargo-ndk.
#
#   android/build.sh [debug|release]      (default release)
#
# A release build is signed with the key named by U1_KEYSTORE,
# U1_KEYSTORE_PASS, U1_KEY_ALIAS (U1_KEY_PASS defaults to the store
# password). Without them it is signed with the Android debug key.
# ABIs: U1_ABIS (default "arm64-v8a x86_64").
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
kind=${1:-release}
abis=${U1_ABIS:-arm64-v8a x86_64}
SDL_VER=2.30.9
SDL_SHA=24b574f71c87a763f50704bbb630cbe38298d544a1f890f099a4696b1d6beba4
min_api=21
cache=$here/.cache
mkdir -p "$cache"

# SDL2's source (its Java activity and the library).
sdl=$cache/SDL2-$SDL_VER
if [ ! -d "$sdl" ]; then
  curl -fsSL -o "$cache/sdl.tgz" "https://github.com/libsdl-org/SDL/releases/download/release-$SDL_VER/SDL2-$SDL_VER.tar.gz"
  echo "$SDL_SHA  $cache/sdl.tgz" | sha256sum -c -
  tar xzf "$cache/sdl.tgz" -C "$cache"
fi
java_dst=$here/app/src/main/java/org/libsdl/app
mkdir -p "$java_dst"
cp "$sdl"/android-project/app/src/main/java/org/libsdl/app/*.java "$java_dst/"

cmake_bin=$(ls -d "$ANDROID_HOME"/cmake/*/bin | tail -1)
for abi in $abis; do
  case $abi in
    arm64-v8a) triple=aarch64-linux-android ;;
    x86_64) triple=x86_64-linux-android ;;
    armeabi-v7a) triple=armv7-linux-androideabi ;;
    *) echo "unknown ABI $abi"; exit 1 ;;
  esac
  out=$here/app/src/main/jniLibs/$abi
  mkdir -p "$out"
  b=$cache/sdl-build-$abi
  if [ ! -f "$b/libSDL2.so" ]; then
    "$cmake_bin/cmake" -S "$sdl" -B "$b" -G Ninja -DCMAKE_MAKE_PROGRAM="$cmake_bin/ninja" \
      -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
      -DANDROID_ABI=$abi -DANDROID_PLATFORM=android-$min_api -DANDROID_STL=c++_static \
      -DCMAKE_BUILD_TYPE=Release -DSDL_STATIC=OFF -DSDL_SHARED=ON -DSDL_TEST=OFF
    "$cmake_bin/cmake" --build "$b"
  fi
  cp "$b/libSDL2.so" "$out/"
  # The engine, linked against that SDL2.
  (cd "$root" && RUSTFLAGS="-L $b" cargo ndk -t $abi -P $min_api build -p u1-android $([ "$kind" = release ] && echo --release))
  prof=$([ "$kind" = release ] && echo release || echo debug)
  tdir=${CARGO_TARGET_DIR:-$root/target}
  cp "$tdir/$triple/$prof/libmain.so" "$out/libmain.so"
done

cd "$here"
if [ "$kind" = release ]; then
  ./gradlew --no-daemon -q assembleRelease
  apk=app/build/outputs/apk/release/app-release.apk
else
  ./gradlew --no-daemon -q assembleDebug
  apk=app/build/outputs/apk/debug/app-debug.apk
fi
echo "built $here/$apk"
