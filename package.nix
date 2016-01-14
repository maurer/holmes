{ rustPlatform, openssl, postgresql }:
with rustPlatform;

buildRustPackage rec {
  name = "holmes";
  src  = ./.;
  buildInputs = [ openssl postgresql ];
  depsSha256 = "";
}
