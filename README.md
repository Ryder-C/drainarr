# drainarr

Drain your \*arr library to a configured disk-usage target.

drainarr watches a filesystem, and when it crosses a target threshold it asks
Radarr/Sonarr to delete the least-recently-relevant items until you're back
under the target. "Least relevant" can be a watch-history signal (currently
[janitorr-stats](https://github.com/Schaka/janitorr-stats)) or, if you don't wire one up,
just import age.

> [!TIP]
> Set `dry_run = true` to log the would-be deletions without calling the APIs.

## Configuration

drainarr reads `config.toml` from its working directory. See
[`config.example.toml`](./config.example.toml) for a fully-commented sample.

```toml
disk_path = "/data/media" # path to stat for usage
target_usage = "85%" # "<n>%" or an absolute size like "18TB"
check_interval = "10m" # disk check duration
min_added_age = "14d" # protect freshly-imported items
dry_run = false

# optional, omit to rank by age only
[stats]
kind = "janitorr"
url  = "http://localhost:8978"


# add as many radarr and sonarr instances as you want!
[[radarr]]
label   = "movies-4k"
url     = "http://localhost:7878"
api_key = "..."

[[sonarr]]
label   = "tv-4k"
url     = "http://localhost:8989"
api_key = "..."

[[radarr]]
label   = "movies"
url     = "http://localhost:7879"
api_key = "..."

[[sonarr]]
label   = "tv"
url     = "http://localhost:8990"
api_key = "..."
```

You need at least one `[[radarr]]` or `[[sonarr]]` instance.

Log verbosity is controlled by `RUST_LOG` (e.g. `RUST_LOG=info,drainarr=debug`).

## Running

### Docker

```yaml
services:
  drainarr:
    image: ghcr.io/ryder-c/drainarr:latest
    container_name: drainarr
    restart: unless-stopped
    volumes:
      - ./config:/config
      # Mount the volume you want drainarr to monitor at the same path
      # referenced by `disk_path` in config.toml.
      - /data/media:/data/media:ro
    environment:
      RUST_LOG: info
```

Put your `config.toml` into the `./config` directory before starting. A
`docker-compose.yml` is included for the same setup.

### NixOS (flake)

```nix
{
  inputs.drainarr.url = "github:Ryder-C/drainarr";

  outputs = { self, nixpkgs, drainarr, ... }: {
    nixosConfigurations.media = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        drainarr.nixosModules.default
        ({ ... }: {
          services.drainarr = {
            enable = true;
            settings = {
              disk_path = "/data/media";
              target_usage = "85%";
              check_interval = "10m";
              min_added_age = "14d";

              stats = {
                kind = "janitorr";
                url = "http://localhost:8978";
              };

              radarr = [{
                label = "movies";
                url = "http://localhost:7878";
                api_key = "REPLACE_ME";
              }];

              sonarr = [{
                label = "tv";
                url = "http://localhost:8989";
                api_key = "REPLACE_ME";
              }];
            };
          };
        })
      ];
    };
  };
}
```

Values in `services.drainarr.settings` are rendered into the world-readable
Nix store. If you'd rather keep API keys out of the store, set
`services.drainarr.configFile` to a path produced by `sops-nix` or `agenix`
and omit `settings`.

## License

MIT
