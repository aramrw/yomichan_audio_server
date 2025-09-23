#!/usr/bin/env nu

# Recursively find all files in the src directory, excluding directories
glob src/**/* --no-dir |
each { |file| open --raw $file } |
save --raw combined.txt --force


