export function closeHttpPreview(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => error ? reject(error) : resolve());
    // Only isolated fixtures: unfinished browser requests must not hold test cleanup open.
    server.closeAllConnections();
  });
}
