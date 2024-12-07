{
  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
    cardboard.url = "github:starptr/cardboard";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    {
      self,
      nixpkgs,
      devenv,
      systems,
      cardboard,
      ...
    }@inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
      metadata = builtins.fromTOML (builtins.readFile ./app/Cargo.toml);
      pname = metadata.package.name;
    in
    {
      packages = forEachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          devenv-up = self.devShells.${system}.default.config.procfileScript;
          ${pname} = cardboard.lib.keepFnInput pkgs.rustPlatform.buildRustPackage {
            inherit pname;
            version = metadata.package.version;
            src = ./app;
            buildInputs =
              [ ]
              ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [
                pkgs.openssl # TODO: check if this is needed for non-darwin targets
                pkgs.darwin.apple_sdk.frameworks.Security
              ];
            cargoLock = {
              lockFile = ./app/Cargo.lock;
            };
          };
          default = self.packages.${system}.${pname};
        }
      );

      overlays = {
        ${pname} = final: prev: {
          ${pname} = self.packages.${final.stdenv.hostPlatform.system}.${pname};
        };
        default = self.overlays.${pname};
      };

      devShells = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [ (import ./devenv.nix) ];
          };
        }
      );

      formatter = forEachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt-rfc-style
      );
    };
}
