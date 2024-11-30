{ pkgs, ... }:
{
  packages = [ pkgs.darwin.apple_sdk.frameworks.Security ];
  languages.rust = {
    enable = true;
  };
}
