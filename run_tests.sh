#!/usr/bin/env bash
mkdir -p $HOME/.holmes
export HOLMES_PG_SOCK_DIR=`tools/pg.bash holmes`
cargo test
rm -rf $HOLMES_PG_SOCK_DIR
