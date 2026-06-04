flake: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.drainarr;

  tomlFormat = pkgs.formats.toml {};

  generatedConfig = tomlFormat.generate "drainarr-config.toml" cfg.settings;

  configFile =
    if cfg.configFile != null
    then cfg.configFile
    else generatedConfig;
in {
  options.services.drainarr = {
    enable = lib.mkEnableOption "drainarr - drain your *arr library to a disk-usage target";

    package = lib.mkOption {
      type = lib.types.package;
      default = flake.packages.${pkgs.stdenv.hostPlatform.system}.drainarr;
      defaultText = lib.literalExpression "drainarr.packages.\${pkgs.stdenv.hostPlatform.system}.drainarr";
      description = "The drainarr package to use.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "drainarr";
      description = "User the drainarr service runs as.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "drainarr";
      description = "Group the drainarr service runs as.";
    };

    configFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      example = lib.literalExpression "\"/run/secrets/drainarr.toml\"";
      description = ''
        Path to a pre-rendered `config.toml`. When set, `settings` is ignored.
        Use this to load a config produced by `sops-nix`, `agenix`, or another
        secret manager so API keys never enter the Nix store.
      '';
    };

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = {};
      example = lib.literalExpression ''
        {
          disk_path = "/data/media";
          target_usage = "85%";
          check_interval = "10m";
          min_added_age = "14d";

          stats = {
            kind = "janitorr";
            url = "http://localhost:8978";
          };

          radarr = [
            {
              label = "movies";
              url = "http://localhost:7878";
              api_key = "REPLACE_ME";
            }
          ];

          sonarr = [
            {
              label = "tv";
              url = "http://localhost:8989";
              api_key = "REPLACE_ME";
            }
          ];
        }
      '';
      description = ''
        Attribute set serialised to `config.toml`. Ignored when
        {option}`services.drainarr.configFile` is set.

        Note that values written here end up in the world-readable Nix store —
        prefer {option}`configFile` for any setting that contains secrets.
      '';
    };

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      example = "info,drainarr=debug";
      description = "Value passed to `RUST_LOG`.";
    };

    readWritePaths = lib.mkOption {
      type = lib.types.listOf lib.types.path;
      default = [];
      example = ["/data/media"];
      description = ''
        Extra paths to expose read-write to the service. drainarr itself only
        reads disk-usage stats and talks to the *arr APIs (which perform the
        on-disk deletions), so this is usually empty.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = lib.mkIf (cfg.user == "drainarr") {
      isSystemUser = true;
      inherit (cfg) group;
      description = "drainarr service user";
    };

    users.groups.${cfg.group} = lib.mkIf (cfg.group == "drainarr") {};

    systemd.services.drainarr = {
      description = "drainarr — drain your *arr library to a disk-usage target";
      wantedBy = ["multi-user.target"];
      after = ["network-online.target"];
      wants = ["network-online.target"];

      environment.RUST_LOG = cfg.logLevel;

      serviceConfig = {
        Type = "simple";
        ExecStartPre = "+${pkgs.writeShellScript "drainarr-prestart" ''
          ${pkgs.coreutils}/bin/install -m 0400 -o ${cfg.user} -g ${cfg.group} \
            ${configFile} "$RUNTIME_DIRECTORY/config.toml"
        ''}";
        ExecStart = lib.getExe cfg.package;
        WorkingDirectory = "/run/drainarr";
        RuntimeDirectory = "drainarr";
        RuntimeDirectoryMode = "0750";
        User = cfg.user;
        Group = cfg.group;
        Restart = "on-failure";
        RestartSec = 30;

        # Hardening
        NoNewPrivileges = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = ["AF_INET" "AF_INET6" "AF_UNIX"];
        RestrictNamespaces = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = ["@system-service" "~@privileged" "~@resources"];
        ReadWritePaths = cfg.readWritePaths;
      };
    };
  };
}
