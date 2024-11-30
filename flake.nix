{
  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
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
      ...
    }@inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      packages = forEachSystem (system: 
        let
          pkgs = import nixpkgs { inherit system; };
          metadata = builtins.fromTOML (builtins.readFile ./app/Cargo.toml);
        in
        {
        devenv-up = self.devShells.${system}.default.config.procfileScript;
        ${metadata.package.name} = pkgs.rustPlatform.buildRustPackage {
          name = metadata.package.name;
          version = metadata.package.version;
          src = ./app;
          buildInputs = [] ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.openssl # TODO: check if this is needed for non-darwin targets
            pkgs.darwin.apple_sdk.frameworks.Security
          ];
          cargoLock = {
            lockFile = ./app/Cargo.lock;
          };
        };
        default = self.packages.${system}.${metadata.package.name};
      });

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
