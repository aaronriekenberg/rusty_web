#!/bin/sh -x

KILL_CMD=killall
CONFIG_FILE=config.yml

if [ $(uname) = 'OpenBSD' ]; then
  KILL_CMD=pkill
  CONFIG_FILE=openbsd-config.yml
fi

$KILL_CMD rusty_web

rm -f output

nohup ./rusty_web $CONFIG_FILE > output 2>&1 &
