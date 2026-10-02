#!/bin/bash
# Builds the decomp before the upgrade (2f4c25d, gc-eu-mq-dbg) for the name map
# (docs/adr/0031-decomp-main.md). Runs inside the oot-old image (the checkout's own Dockerfile),
# from scripts/run/name-map.bat:
#   /src     the old checkout, read only (cloned into the volume, so it stays as it is)
#   /roms    the folder holding baserom.z64, read only
#   /oot     the oot-old volume: the clone and its build
#   /out     where old.elf goes
set -e
git config --global --add safe.directory '*'
if [ ! -d /oot/repo ]; then
  git clone -q /src /oot/repo
fi
cd /oot/repo
git checkout -q 2f4c25da53b3a23251a6f2ab477d588b4da38475
git log -1 --format='%H %s'
cp /roms/baserom.z64 baserom_original.z64
make -j"$(nproc)" setup
make -j"$(nproc)"
cp zelda_ocarina_mq_dbg.elf /out/old.elf
echo BUILD_OLD_DONE
