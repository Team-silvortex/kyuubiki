import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";
import { createConnection } from "node:net";
import test from "node:test";
import { closeHttpPreview } from "./http-preview-lifecycle.shared.mjs";

test("isolated HTTP preview cleanup closes unfinished browser connections", { timeout: 10_000 }, async () => {
  const server = createServer((_request, response) => response.end("preview"));
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const accepted = once(server, "connection");
  const socket = createConnection({ host: "127.0.0.1", port: server.address().port });
  let closed;
  let timer;
  try {
    await once(socket, "connect");
    await accepted;
    socket.write("GET / HTTP/1.1\r\nHost: preview\r\n");
    closed = closeHttpPreview(server);
    await Promise.race([
      closed,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error("preview cleanup waited for a browser socket")), 2_000);
      }),
    ]);
    assert.equal(server.listening, false);
    assert.equal(await new Promise((resolve, reject) => server.getConnections(
      (error, count) => error ? reject(error) : resolve(count),
    )), 0);
  } finally {
    clearTimeout(timer);
    socket.destroy();
    if (closed) await closed;
    else await closeHttpPreview(server);
  }
});
