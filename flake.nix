{
  nixConfig = {
    extra-substituters = [
      "https://claude-code.cachix.org"
    ];
    extra-trusted-public-keys = [
      "claude-code.cachix.org-1:YeXf2aNu7UTX8Vwrze0za1WEDS+4DuI2kVeWEE4fsRk="
    ];
    connect-timeout = 5;
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    claude-code = {
      url = "github:sadjow/claude-code-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      naersk,
      claude-code,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
            android_sdk.accept_license = true;
          };
        };

        # ── Toolchain ─────────────────────────────────────────────
        rust = pkgs.rust-bin.nightly.latest.default;

        naersk' = pkgs.callPackage naersk {
          cargo = rust;
          rustc = rust;
        };

        # ── Build helper ──────────────────────────────────────────
        buildApp =
          { release }:
          naersk'.buildPackage {
            name = "bp-to-bagels-csv";
            src = ./.;
            inherit release;

            meta = with pkgs.lib; {
              description = "Import a Banque Populaire CSV export into a Bagels SQLite database.";
              homepage = "https://github.com/wallago/bp-to-bagels-csv";
              license = [
                licenses.mit
                licenses.asl20
              ];
            };
          };

        # ── Claude Settings ─────────────────────────────────────
        claude = claude-code.packages.${system}.default;
        claudeLocalSettings = builtins.toJSON {
          permissions = {
            allow = [
              # Nix
              "Bash(nix flake check*)"
              "Bash(nix eval*)"
              "Bash(nixos-rebuild dry-build*)"
              "Bash(statix check*)"
              "Bash(deadnix*)"
              "Bash(just*)"
              "Bash(nix build --dry-run*)"
              "Bash(nix search nixpkgs*)"
              "Bash(curl -s https://search.nixos.org*)"

              # Rust
              "Bash(cargo check*)"
              "Bash(cargo clippy*)"
              "Bash(cargo nextest run*)"
              "Bash(cargo test*)"
              "Bash(cargo tree*)"
              "Bash(cargo machete*)"
              "Bash(cargo deny check*)"
              "Bash(cargo audit*)"
            ];
          };
          enabledPlugins = {
            "claude-md-management@claude-plugins-official" = true;
            "superpowers@claude-plugins-official" = true;
            "context7@claude-plugins-official" = true;
            "code-review@claude-plugins-official" = true;
            "code-simplifier@claude-plugins-official" = true;
            "github@claude-plugins-official" = true;
          };
        };
      in
      rec {
        # ── Packages ──────────────────────────────────────────────
        packages = rec {
          bp-to-bagels-csv = buildApp { release = true; };
          bp-to-bagels-csv-debug = buildApp { release = false; };
          default = bp-to-bagels-csv;
        };

        # ── Checks (nix flake check) ─────────────────────────────
        checks.check = packages.rssh-debug;

        # ── Dev Shell (nix develop) ──────────────────────────────
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust
            rust-analyzer
            just
            claude
            nodejs

            # crate deps
            sqlite
          ];
          shellHook = ''
            mkdir -p .claude
            echo '${claudeLocalSettings}' > .claude/settings.local.json
          '';
        };
      }
    );
}
