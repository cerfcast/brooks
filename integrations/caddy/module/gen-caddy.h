#!/bin/env bash 

src=`pwd`

echo ${src}

tmpdir=`mktemp -d`
cd ${tmpdir}

go tool cgo -importpath github.com/brooks/brooks -srcdir=${src}/brooks/ -- -O2 -g -I${src}/brooks/shim/ ffi.go log.go

cp _obj/_cgo_export.h ${src}/caddy-raw.h

cd ${src}

rm -rf ${tmpdir}
