#!/usr/bin/env bash
#
# Rewrite the version key inside Cargo.toml's [package] table, leaving version
# keys in every other table (dependencies, etc.) untouched.
#
# Usage: set-cargo-version.sh X.Y.Z

set -euo pipefail

version="${1:?usage: set-cargo-version.sh X.Y.Z}"

awk -v v="$version" '
	/^\[/ { in_pkg = ($0 ~ /^\[package\]/) }
	in_pkg && !done && /^[[:space:]]*version[[:space:]]*=/ {
		print "version = \"" v "\""
		done = 1
		next
	}
	{ print }
	END { if (!done) { print "error: no version key found in [package]" > "/dev/stderr"; exit 1 } }
' Cargo.toml >Cargo.toml.tmp

mv Cargo.toml.tmp Cargo.toml
