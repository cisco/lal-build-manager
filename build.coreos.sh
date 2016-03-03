#!/bin/bash
cargo build --release
mkdir -p coreos/bin
mkdir -p coreos/lib
cp target/release/lal coreos/bin
ldd coreos/bin/lal  | awk '{ print $3 }' | grep -E "." | xargs cp -t coreos/lib
cd coreos/lib
rm libc.so* \
   libcom_err.so* \
   libdl.so* \
   libffi.so* \
   libgcc_s.so* \
   libkeyutils.so* \
   libm.so* \
   libpthread.so* \
   libresolv.so* \
   libz.so*

cd -
tar czf lal.coreos.tar -C coreos .
rm -rf coreos/
