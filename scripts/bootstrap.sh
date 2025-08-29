#!/bin/sh

SCRIPTDIR=$(dirname $(readlink -f "$0"))
BASEDIR=$(dirname "${SCRIPTDIR}")
LOCALDIR="${BASEDIR}/local"
BINDIR="${LOCALDIR}/bin"

if [ ! -f "${BINDIR}/wasm-pack" ]; then
    cargo install --root "${LOCALDIR}" wasm-pack
fi
