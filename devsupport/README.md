# Devsupport - Docker Compose

Docker Compose configuration for local development.

## Services

Currently, the following services are available:

- [x] mux
- [x] encoder

See `compose.yml` for used ports etc.

## Usage

```shell
docker compose up --build # -d
```


## Notes

Supervisor console

```shell
docker compose exec -u root encoder supervisorctl
```
