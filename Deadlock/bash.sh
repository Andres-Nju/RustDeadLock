#!/bin/bash

# check arg
if [ -z "$1" ]; then
  echo "Usage: $0 <input_dir>"
  exit 1
fi

input_dir=$1

cargo install --path .
cd "../test/$input_dir" || { echo "Directory ../test/$input_dir not found."; exit 1; }
call-deadlock
cd ../../Deadlock || { echo "Directory ../../Deadlock not found."; exit 1; }