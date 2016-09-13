{ nixpkgs ? import <nixpkgs> {} }:

nixpkgs.callPackage (import ./package.nix) {}
