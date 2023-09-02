{ self, lib, inputs, ... }:
{
  perSystem = { config, self', pkgs, system, ... }:
    {
      # e2e test service using playwright
      process-compose.cargo-e2e-playwright-test =
        let
          TEST_PORT = "5000";
        in
        {
          tui = false;
          port = 8975;
          settings.processes = {
            start-app = {
              command = "${lib.getExe self'.packages.default} --site-addr=127.0.0.1:${TEST_PORT}";
              readiness_probe = {
                exec.command = "${lib.getExe pkgs.curl} --fail 127.0.0.1:${TEST_PORT}";
                initial_delay_seconds = 2;
                period_seconds = 10;
                timeout_seconds = 4;
              };
            };
            test = {
              environment = {
                inherit TEST_PORT;
              };
              command = pkgs.writeShellApplication {
                name = "e2e-playwright";
                runtimeInputs = with pkgs; [ nodejs playwright-test ];
                text = ''
                  cd e2e-playwright
                  playwright test --project chromium
                '';
              };
              depends_on."start-app".condition = "process_healthy";
            };
          };
        };
    };
}