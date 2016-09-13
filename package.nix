{ rustPlatform, openssl, postgresql }:
with rustPlatform;

buildRustPackage rec {
  name = "holmes";
  src  = ./.;
  buildInputs = [ openssl postgresql ];
  checkPhase = ''
    echo "Bringing up postgres server"
    export HOLMES_PG_SOCK_DIR=`tools/pg.bash holmes`
    echo "Running cargo test"
    cargo test
  '';
  depsSha256 = "";
}
