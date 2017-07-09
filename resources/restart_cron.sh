#!/bin/sh

cd /home/aaron/rusty_web_run

if [ $(uname) = 'OpenBSD' ]; then
  pgrep -q rusty_web
  if [ $? -eq 1 ]; then
    ./restart.sh > /dev/null 2>&1
  fi
else
  echo 'not implemented'
  exit 1
fi
