#!/bin/bash
# Builds the pinned decomp (zeldaret/oot main at 52a510f, gc-eu-mq-dbg) for the name map
# (docs/adr/0031-decomp-main.md). Runs inside the oot-new image (the checkout's own Dockerfile),
# from scripts/run/name-map.bat:
#   /src     the new checkout, read only (cloned into the volume, so it stays as it is)
#   /roms    the folder holding baserom.z64, read only
#   /oot     the oot-new volume: the clone and its build
#   /out     where new.elf goes
set -e
git config --global --add safe.directory '*'
if [ ! -d /oot/repo ]; then
  git clone -q /src /oot/repo
fi
cd /oot/repo
git checkout -q 52a510f379afd143aaa0375be9f1e190369572e1
git log -1 --format='%H %s'
cp /roms/baserom.z64 baseroms/gc-eu-mq-dbg/baserom.z64
make -j"$(nproc)" setup VERSION=gc-eu-mq-dbg
make -j"$(nproc)" VERSION=gc-eu-mq-dbg
cp build/gc-eu-mq-dbg/oot-gc-eu-mq-dbg.elf /out/new.elf
echo BUILD_NEW_DONE
