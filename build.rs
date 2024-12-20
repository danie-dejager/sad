#!/usr/bin/env -S -- bash -Eeuo pipefail
// || rustc --edition=2021 -o "${T:="$(mktemp)"}" -- "$0" && exec -a "$0" -- "$T" "$0" "$@"
#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::wildcard_dependencies)]

fn main() {
  println!("cargo:rustc-env=SAD_ARGV_UUID=4f3828bf-ebf6-4f46-b07b-7eb9e4ae4380");

  println!("cargo:rustc-env=SAD_PREVIEW_UUID=b477f4c9-9fe7-4224-92cd-1632521ec2f0");

  println!("cargo:rustc-env=SAD_PATCH_UUID=cadfe8eb-0dae-46be-bb4a-a058330e62a4");
}
