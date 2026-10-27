#!/bin/bash
# Builds yue2.cpp's `yue-server` at the pinned commit and copies it, with its
# ggml libraries, into src-tauri/resources/yue2-cpp/ where Cadenza (dev and
# bundled builds) finds it.
#   macOS: Metal.   Linux: Vulkan (NVIDIA, AMD, Intel), CUDA with ENGINE_BACKEND=cuda.
# Windows uses scripts/build-engine.ps1.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
repository="https://github.com/timoncool/yue2.cpp.git"
commit="ec43393ef2c7f604cf0b054911eb6172eec5a6ee"
worktree="${ENGINE_BUILD_ROOT:-$root/engine-build}/yue2.cpp"
output="$root/src-tauri/resources/yue2-cpp"
build="$worktree/build"

if [ ! -d "$worktree/.git" ]; then
    mkdir -p "$(dirname "$worktree")"
    git clone "$repository" "$worktree"
fi
git -C "$worktree" fetch --quiet origin "$commit" || git -C "$worktree" fetch --quiet origin
git -C "$worktree" checkout --quiet "$commit"
git -C "$worktree" submodule update --init --recursive

flags=(-DCMAKE_BUILD_TYPE=Release -DCMAKE_BUILD_WITH_INSTALL_RPATH=ON -DGGML_NATIVE=OFF)
case "$(uname -s)" in
    Darwin)
        flags+=(-DCMAKE_INSTALL_RPATH=@executable_path -DGGML_METAL=ON -DGGML_METAL_EMBED_LIBRARY=ON)
        ;;
    *)
        # the libraries are found beside the binary, wherever the package puts them
        flags+=(-DCMAKE_INSTALL_RPATH='$ORIGIN' -DCMAKE_BUILD_RPATH='$ORIGIN')
        case "${ENGINE_BACKEND:-vulkan}" in
            cuda)   flags+=(-DGGML_CUDA=ON "-DCMAKE_CUDA_ARCHITECTURES=75-real;80-real;86-real;89-real;90-real") ;;
            vulkan) flags+=(-DGGML_VULKAN=ON) ;;
            cpu)    ;;
        esac
        ;;
esac
if command -v ninja >/dev/null; then flags+=(-G Ninja); fi

cmake -S "$worktree" -B "$build" "${flags[@]}"
cmake --build "$build" --config Release --target yue-server -j "$(getconf _NPROCESSORS_ONLN)"

server="$(find "$build" -maxdepth 3 -type f -name yue-server | head -n 1)"
[ -n "$server" ] || { echo "[ERROR] the build finished without yue-server" >&2; exit 1; }

mkdir -p "$output"
find "$output" -mindepth 1 ! -name .keep -delete
cp "$server" "$output/"
# -a keeps the version symlinks of the shared libraries the binary links through its rpath
find "$build" -maxdepth 3 \( -name 'lib*.dylib' -o -name 'lib*.so*' \) -exec cp -a {} "$output/" \;
echo "$commit" > "$output/runtime.commit"
echo "yue-server ($commit) is in $output:"
ls -la "$output"
