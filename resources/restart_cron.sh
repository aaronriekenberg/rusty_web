#!/bin/sh

if [ $(uname) = 'OpenBSD' ]; then
  pgrep -q rusty_web
  if [ $? -eq 1 ]; then
    cd /home/aaron/rusty_web_run
    ./restart.sh > /dev/null 2>&1
  fi
else
  pgrep rusty_web > /dev/null 2>&1
  if [ $? -eq 1 ]; then
    cd /home/pi/rusty_web_run
    ./restart.sh > /dev/null 2>&1
  fi
fi
