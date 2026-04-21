# Logios examples

Runnable examples for [`@logios/sdk`](../sdk/ts) plus raw `curl` recipes for the
public `/v1` API.

## TypeScript

These use [`tsx`](https://www.npmjs.com/package/tsx) to run TypeScript directly.

```sh
# from the repo root
npm install @logios/sdk
npm install -D tsx
```

| File | What it does |
| --- | --- |
| [`quickstart.ts`](./quickstart.ts) | Connect, print stats + the latest slot, and narrate it. |
| [`watch-slots.ts`](./watch-slots.ts) | Poll the head every 2s and narrate each new sealed slot. |

```sh
npx tsx examples/quickstart.ts
npx tsx examples/watch-slots.ts   # ctrl-c to stop
```

## curl

[`curl.md`](./curl.md) has copy-pasteable `curl` commands for every `/v1`
endpoint, with example responses.

## CLI

Prefer a terminal tool? The [`logios` CLI](../cli) wraps the same SDK:

```sh
npx @logios/cli status
npx @logios/cli watch
npx @logios/cli explain 4542500
```
