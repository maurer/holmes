{ nixpkgs ? import <nixpkgs> {}}:
nixpkgs.callPackage (
{ stdenv, rust, postgresql, openssl, vim_configurable }:

# Vim with rust + git support
let vim = vim_configurable.customize {
  name = "vim";
  vimrcConfig.customRC = ''
    set backspace=indent,eol,start

    let g:rustfmt_autosave = 1
    autocmd FileType rust compiler cargo
    let g:racer_cmd = "${nixpkgs.rustracer}/bin/racer"
    let $RUST_SRC_PATH="${nixpkgs.rustc.src}/src"
    let g:racer_experimental_completer = 1
    au FileType rust nmap gd <Plug>(rust-def)
    au FileType rust nmap gs <Plug>(rust-def-split)
    au FileType rust nmap gx <Plug>(rust-def-vertical)
    au FileType rust nmap <leader>gd <Plug>(rust-doc)
  '';
  vimrcConfig.vam.pluginDictionaries = [{ names = [
    "fugitive"
    "vim-racer"
    "rust-vim"
    "syntastic"
  ];}];
}; in

stdenv.mkDerivation rec {
  name = "bap-rust";
  buildInputs = [ rust postgresql openssl vim ];
}
) {rust = nixpkgs.rustChannels.stable.rust; }
