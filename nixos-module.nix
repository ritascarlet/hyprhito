{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.hyprhito;
  hyprhito = pkgs.callPackage ./default.nix {};
in {
  
  options.services.hyprhito = {
    enable = mkEnableOption "hyprhito laptop lid display manager";
  };

  config = mkIf cfg.enable {
    
    environment.systemPackages = [ hyprhito ];
    
    services.logind = {
      lidSwitch = "ignore";
      lidSwitchDocked = "ignore";
      lidSwitchExternalPower = "ignore";
    };
    
    systemd.services.hyprhito = {
      description = "Hyprhito - Low-level laptop lid display manager";
      documentation = [ "https://github.com/hito/hyprhito" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "multi-user.target" ];
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${hyprhito}/bin/hyprhito";
        Restart = "always";
        RestartSec = 3;
        
        User = "root";
        Group = "root";
        
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectHome = true;
        ProtectKernelTunables = false;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;
        
        Environment = "RUST_LOG=info";
        StandardOutput = "journal";
        StandardError = "journal";
      };
    };
  };
}
