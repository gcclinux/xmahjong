#!/usr/bin/env bash
#
# Build (debug or release) and run xMahjong on Linux.
#
# Usage:
#   ./run.sh                    # debug build, normal run
#   ./run.sh --release          # release build
#   ./run.sh --dev --level 29   # dev mode, start at level 29
#   ./run.sh --dev --level 50   # dev mode, start at level 50

set -e

RELEASE=0
DEV=0
LEVEL=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release|-r)
            RELEASE=1
            shift
            ;;
        --dev|-d)
            DEV=1
            shift
            ;;
        --level|-l)
            LEVEL="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./run.sh [--release] [--dev] [--level N]"
            exit 1
            ;;
    esac
done

# Build
if [[ $RELEASE -eq 1 ]]; then
    echo -e "\033[36mBuilding (release)...\033[0m"
    cargo build --release
    PROFILE="release"
else
    echo -e "\033[36mBuilding (debug)...\033[0m"
    cargo build
    PROFILE="debug"
fi

# Run
EXE="target/$PROFILE/xmahjong"

RUN_ARGS=()
if [[ $DEV -eq 1 ]]; then
    RUN_ARGS+=("--dev")
    if [[ $LEVEL -gt 0 ]]; then
        RUN_ARGS+=("--level" "$LEVEL")
    fi
    echo -e "\033[33mRunning (DEV mode, level $LEVEL): $EXE ${RUN_ARGS[*]}\033[0m"
else
    echo -e "\033[32mRunning: $EXE\033[0m"
fi

exec "$EXE" "${RUN_ARGS[@]}"
