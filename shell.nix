{ nixpkgs ? import <nixpkgs> {}}:
nixpkgs.callPackage (
{ stdenv, rust, openssl, postgresql }:
with rust;
stdenv.mkDerivation rec {
  name = "holmes";
  buildInputs = [ cargo rustc
                  openssl postgresql ];
}
) {}
