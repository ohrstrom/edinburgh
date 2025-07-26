# Devsupport - Docker Compose

Docker Compose configuration for local development.

## Services

Currently, the following services are available:

- [ ] mux
- [ ] encoder

See `compose.yml` for used ports etc.

## Usage

```shell
docker compose --build up # -d
```


## Notes

Supervisor console

```shell
docker compose exec -u root encoder supervisorctl
```
