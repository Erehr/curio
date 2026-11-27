# Client

The client for Curio's [web API](https://erehr.github.io/curio/api/connect/http/), for userscripts, browser extensions, web pages on this computer and Node; plugin pages get the same file. Like the rest of Curio, it is under the GNU GPL version 3 (`LICENSING.md` at the top of the source).

| | |
|---|---|
| [`curio.js`](curio.js) | The client; the app serves it at `http://127.0.0.1:41733/api/v1/curio.js`. See `docs/api/connect/client.md`. |
| [`curio.d.ts`](curio.d.ts) | Its types, made from the operation registry; don't edit by hand (`curio-dev reference` in Curio's development tools writes it). |
