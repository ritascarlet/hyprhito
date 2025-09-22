{
  description = "hyprhito - утилита для управления дисплеем при закрытии крышки ноутбука";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      # Общие функции для создания пакета
      mkHyprhito = pkgs: pkgs.callPackage ./default.nix {};
      
      # NixOS модуль
      nixosModules.hyprhito = import ./nixos-module.nix;
      
      # Модуль для home-manager (устарел - используйте NixOS модуль)
      homeManagerModules.hyprhito = { config, lib, pkgs, ... }:
        with lib;
        let cfg = config.services.hyprhito;
        in {
          options.services.hyprhito = {
            enable = mkEnableOption "hyprhito laptop lid display manager (deprecated - use NixOS module instead)";
          };
          
          config = mkIf cfg.enable {
            # Предупреждение о том, что модуль устарел
            warnings = [
              "services.hyprhito в home-manager устарел. Используйте NixOS модуль для системного сервиса."
            ];
            
            # Простое добавление пакета без сервиса
            home.packages = [ (mkHyprhito pkgs) ];
          };
        };
        
    in
    # Per-system outputs
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        hyprhito = mkHyprhito pkgs;
      in
      {
        packages = {
          default = hyprhito;
          hyprhito = hyprhito;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            
            # Development tools
            pkg-config
            
            # System libraries
            systemd
            
            # Debugging tools
            strace
            gdb
          ];

          RUST_BACKTRACE = "1";
          
          shellHook = ''
            echo "🦀 Добро пожаловать в окружение разработки hyprhito!"
            echo "📦 Rust версия: $(rustc --version)"
            echo "⚙️  Cargo версия: $(cargo --version)"
            echo ""
            echo "💡 Доступные команды:"
            echo "   cargo build          - Собрать проект"
            echo "   cargo run            - Запустить проект"
            echo "   cargo test           - Запустить тесты"
            echo "   cargo clippy         - Анализ кода"
            echo "   cargo fmt            - Форматирование кода"
            echo "   nix build            - Собрать пакет через Nix"
            echo "   nix run              - Запустить через Nix"
            echo ""
          '';
        };

        apps.default = flake-utils.lib.mkApp {
          drv = hyprhito;
        };
      }))
    //
    # Global outputs
    {
      # NixOS модули
      inherit nixosModules;
      
      # Home Manager модули  
      inherit homeManagerModules;
      
      # Overlay для добавления пакета в nixpkgs
      overlays.default = final: prev: {
        hyprhito = mkHyprhito final;
      };
    };
}
