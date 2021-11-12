#!/bin/sh

cargo build --target i686-unknown-linux-gnu

g++ -shared -m32 -o voiceserver.ext.2.csgo.so -Wl,--whole-archive ./target/i686-unknown-linux-gnu/debug/libvoiceserver_ext.a -Wl,--no-whole-archive \
	$HL2SDKCSGO/lib/linux/tier1_i486.a \
	$HL2SDKCSGO/lib/linux/mathlib_i486.a \
	$HL2SDKCSGO/lib/linux/interfaces_i486.a \
	$HL2SDKCSGO/lib/linux32/release/libprotobuf.a \
	-L$HL2SDKCSGO/lib/linux/ -ltier0 -lvstdlib
