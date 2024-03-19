#!/bin/sh

cd /code
rm -rf kakarot-indexer
git clone -v https://github.com/kkrt-labs/kakarot-indexer.git

exec "$@"
