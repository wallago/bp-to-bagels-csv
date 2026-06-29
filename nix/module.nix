self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.bp-to-bagels-csv;
in
{
  options.programs.bp-to-bagels-csv = {
    enable = lib.mkEnableOption "bp-to-bagels-csv";
    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "The bp-to-bagels-csv package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
