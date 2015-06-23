{ rustPlatform, openssl, capnproto, which, postgresql }:
with rustPlatform;

buildRustPackage rec {
  name = "holmes";
  src  = ./.;
  buildInputs = [ openssl capnproto which postgresql ];
  depsSha256 = "";
}
