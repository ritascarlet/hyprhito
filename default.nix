{ lib
, rustPlatform
, pkg-config
, systemd
}:

rustPlatform.buildRustPackage rec {
  pname = "hyprhito";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    systemd
  ];

  doCheck = false;

  postInstall = ''
    mkdir -p $out/share/hyprhito
    
    cat > $out/share/hyprhito/README << EOF
# hyprhito - Low-level laptop lid display manager

## Installation via NixOS module:

Add to configuration.nix:
\`\`\`nix
{
  imports = [ ./path/to/hyprhito/nixos-module.nix ];
  
  services.hyprhito.enable = true;
}
\`\`\`

## Service management:
- Status: sudo systemctl status hyprhito
- Logs: sudo journalctl -fu hyprhito  
- Restart: sudo systemctl restart hyprhito
- Enable: sudo systemctl enable hyprhito
EOF
  '';

  meta = with lib; {
    description = "Low-level laptop lid display manager";
    longDescription = ''
      Rust service that automatically manages display state when laptop lid is
      opened/closed. Uses low-level monitoring of input events directly from
      kernel via /dev/input.
      
      Features:
      - Low-level monitoring via evdev (/dev/input/event*)
      - Lightning-fast response to lid events
      - Direct DRM sysfs control without sudo
      - ARM64 and non-ACPI system support
      - Systemd integration as system service
      - Security through systemd hardening
    '';
    homepage = "https://github.com/hito/hyprhito";
    license = licenses.mit;
    maintainers = [ "hito" ];
    platforms = platforms.linux;
    mainProgram = "hyprhito";
  };
}
