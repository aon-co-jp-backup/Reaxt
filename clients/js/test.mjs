// 実サーバー(examples/counter、8765)に対する試験: node test.mjs
import { createClient, ReaxtError } from "./reaxt-client.mjs";
import assert from "node:assert/strict";
const c = createClient({ baseUrl: process.env.REAXT_URL ?? "http://127.0.0.1:8765" });
const list = await c.manifest();
assert.ok(list.some((a) => a.path === "/demo.v1.Counter/Inc"));
const inc = c.defineAction("demo.v1.Counter", "Inc");
const a = await inc(), b = await inc();
assert.equal(b, a + 1);
await assert.rejects(c.call("demo.v1.Counter", "Nope"), (e) => e instanceof ReaxtError);
const noHeader = createClient({ baseUrl: process.env.REAXT_URL ?? "http://127.0.0.1:8765", headers: { "x-reaxt": "" } });
await assert.rejects(noHeader.call("demo.v1.Counter", "Inc"), (e) => e.status === 403 && e.code === "permission_denied");
console.log("ok", a, b);
