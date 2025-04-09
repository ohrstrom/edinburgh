# EDInburgh WASM

ehlo..

```shell
wasm-pack build
```

```shell
cd ui

bun install
bun dev --port 3001
```


```shell
# chrome wit separate profile
open -na 'Google Chrome' --args --user-data-dir="${PWD}/ui/tmp/chrome-profile" 'http://localhost:3001'
```
