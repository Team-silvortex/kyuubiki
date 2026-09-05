import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import {
  KYUUBIKI_PRODUCT_VERSION,
  KYUUBIKI_PRODUCT_VERSION_LABEL,
} from "../../src/lib/product-version.ts";
import { WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION } from "../../src/lib/workbench/helpers.ts";

test("frontend product identity follows the package manifest", async () => {
  const packageUrl = new URL("../../package.json", import.meta.url);
  const metadata = JSON.parse(await readFile(packageUrl, "utf8")) as { version: string };

  assert.equal(KYUUBIKI_PRODUCT_VERSION, metadata.version);
  const brandUrl = new URL("../../public/brand.json", import.meta.url);
  const brand = JSON.parse(await readFile(brandUrl, "utf8"));
  assert.equal(brand.releaseVersion, metadata.version);
  assert.equal(KYUUBIKI_PRODUCT_VERSION_LABEL, `${brand.releaseCodename} ${metadata.version}`);
  assert.equal(WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION, metadata.version);
});
