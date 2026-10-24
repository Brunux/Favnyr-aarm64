{
  description = "Favnyr - File manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        favnyr = pkgs.callPackage ./package.nix { };
      in
      {
        packages = {
          default = favnyr;
        };

        # Without this, `nix flake check` has nothing to check and passes on a
        # recipe that does not build.
        checks = {
          default = favnyr;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = favnyr;
            exePath = "/bin/favnyr";
          };
        };
      }
    );
}