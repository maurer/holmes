#!/usr/bin/env bash
export PGDATA=`mktemp -d -p /anvil/data`
pg_ctl initdb -s -o -Atrust
echo "unix_socket_directories = '$PGDATA'" >> $PGDATA/postgresql.conf
echo "listen_addresses = ''" >> $PGDATA/postgresql.conf
echo "work_mem = 1GB" >> $PGDATA/postgresql.conf
echo "synchronous_commit = off" >> $PGDATA/postgresql.conf
pg_ctl -w start -s -l/dev/null
createuser -h $PGDATA -s $1
echo $PGDATA
