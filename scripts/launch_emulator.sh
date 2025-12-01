#!/bin/bash

# $1 = path to emulator executable
# $2 = path to file to run
# $@ additional arguments to emulator

emulator="$1"
#shift
#file="$1"
#shift

# detach completely from terminal/session
#nohup "$emulator" "$file" "$@" > /dev/null 2>&1 &
nohup "$emulator" > /dev/null 2>&1 &
disown

