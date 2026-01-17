#!/usr/bin/env bash
#
# Wrapper script for cargo embed with board selection support
#
# Usage:
#   ./embed.sh [--release]
#   BOARD=microbit ./embed.sh [--release]
#   ./embed.sh --chip microbit [--release]
#
# Board selection priority (highest to lowest):
# 1. --chip command line argument
# 2. BOARD environment variable
# 3. Default (feather)
#

set -euo pipefail

# Default board
BOARD_CHIP="${BOARD:-feather}"

# Parse arguments to check for --chip override
CARGO_ARGS=()
SKIP_NEXT=false

for arg in "$@"; do
    if [ "$SKIP_NEXT" = true ]; then
        BOARD_CHIP="$arg"
        SKIP_NEXT=false
        continue
    fi
    
    if [ "$arg" = "--chip" ]; then
        SKIP_NEXT=true
        continue
    fi
    
    CARGO_ARGS+=("$arg")
done

echo "Building and flashing board: $BOARD_CHIP"
exec cargo embed --chip "$BOARD_CHIP" "${CARGO_ARGS[@]}"
