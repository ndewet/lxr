#!/usr/bin/env bash
#
# Rewrite the version keys inside the root Cargo.toml, leaving version keys in
# every other table untouched. Every crate of the workspace inherits the
# version from [workspace.package], and [workspace.dependencies] pins the path
# dependencies between them at the same version.
#
# Inside [workspace.dependencies], rewrite only an entry that carries a `path`
# key. Such an entry names a crate of this workspace, thus its version tracks
# the version of the workspace. An entry with no `path` names a crate of
# another author, and its version must stay as it is.
#
# Usage: set-cargo-version.sh X.Y.Z

set -euo pipefail

version="${1:?usage: set-cargo-version.sh X.Y.Z}"

awk -v v="$version" '
	/^\[/ {
		in_pkg = ($0 ~ /^\[workspace\.package\]/)
		in_deps = ($0 ~ /^\[workspace\.dependencies\]/)
	}
	in_pkg && !done && /^[[:space:]]*version[[:space:]]*=/ {
		print "version = \"" v "\""
		done = 1
		next
	}
	in_deps && /path[[:space:]]*=/ {
		gsub(/version[[:space:]]*=[[:space:]]*"[^"]*"/, "version = \"" v "\"")
	}
	{ print }
	END { if (!done) { print "error: no version key found in [workspace.package]" > "/dev/stderr"; exit 1 } }
' Cargo.toml >Cargo.toml.tmp

mv Cargo.toml.tmp Cargo.toml
