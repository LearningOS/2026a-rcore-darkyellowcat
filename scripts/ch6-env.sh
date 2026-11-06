#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "Usage: bash scripts/ch6-env.sh <command> [args...]" >&2
    exit 2
fi

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
git_common_dir="$(git -C "$project_dir" rev-parse --path-format=absolute --git-common-dir)"
mkdir -p "$project_dir/tmp/ch6-env/cargo"

docker_args=(--rm --interactive --user "$(id -u):$(id -g)")
if [[ -t 0 && -t 1 ]]; then
    docker_args+=(--tty)
fi

docker run "${docker_args[@]}" \
    --network host \
    --mount "type=bind,src=$project_dir,dst=$project_dir" \
    --mount "type=bind,src=$git_common_dir,dst=$git_common_dir,readonly" \
    --workdir "$project_dir" \
    --env CARGO_HOME="$project_dir/tmp/ch6-env/cargo" \
    --env RUSTUP_HOME=/usr/local/rustup \
    --env RUSTUP_TOOLCHAIN=nightly-2024-05-02 \
    rcore-docker-qemu8 "$@"
