#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);

const schemaContracts = [
  {
    config: "config/operator-reliability-manifest.json",
    schema: "schemas/operator-reliability-manifest.schema.json",
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: "config/operator-qualification-roadmap.json",
    schema: "schemas/operator-qualification-roadmap.schema.json",
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: "config/operator-qualification-evidence-kits.json",
    schema: "schemas/operator-qualification-evidence-kits.schema.json",
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
];

function fail(message) {
  console.error(`operator reliability schema check failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function schemaValueAt(schema, valuePath) {
  return valuePath.reduce((value, key) => value?.[key], schema);
}

function requiredFieldErrors(value, schema, context) {
  const errors = [];
  if (!schema || typeof schema !== "object") {
    return errors;
  }

  if (Array.isArray(schema.required) && value && typeof value === "object") {
    for (const field of schema.required) {
      if (!(field in value)) {
        errors.push(`${context}: missing required field ${field}`);
      }
    }
  }

  if (schema.properties && value && typeof value === "object" && !Array.isArray(value)) {
    for (const [field, fieldSchema] of Object.entries(schema.properties)) {
      if (field in value) {
        errors.push(...requiredFieldErrors(value[field], fieldSchema, `${context}.${field}`));
      }
    }
  }

  if (schema.items && Array.isArray(value)) {
    value.forEach((item, index) => {
      errors.push(...requiredFieldErrors(item, schema.items, `${context}[${index}]`));
    });
  }

  return errors;
}

function checkRequiredFields(value, schema, context) {
  const errors = requiredFieldErrors(value, schema, context);
  if (errors.length > 0) {
    fail(errors[0]);
  }
}

function checkSchemaContract({ config, schema, schemaVersionPath }) {
  const configJson = readJson(config);
  const schemaJson = readJson(schema);
  checkRequiredFields(configJson, schemaJson, config);
  const expectedSchemaVersion = schemaValueAt(schemaJson, schemaVersionPath);
  if (!expectedSchemaVersion) {
    fail(`${schema}: missing schema_version const`);
  }
  if (configJson.schema_version !== expectedSchemaVersion) {
    fail(`${config}: schema_version must match ${schema}`);
  }
}

function checkReliabilityShards() {
  const manifest = readJson("config/operator-reliability-manifest.json");
  const shardSchema = readJson("schemas/operator-reliability-shard.schema.json");
  const expectedSchemaVersion = schemaValueAt(shardSchema, [
    "properties",
    "schema_version",
    "const",
  ]);
  if (!expectedSchemaVersion) {
    fail("schemas/operator-reliability-shard.schema.json: missing schema_version const");
  }
  if (!Array.isArray(manifest.shards) || manifest.shards.length === 0) {
    fail("config/operator-reliability-manifest.json: shards must be non-empty");
  }
  for (const shardPath of manifest.shards) {
    const shard = readJson(shardPath);
    checkRequiredFields(shard, shardSchema, shardPath);
    if (shard.schema_version !== expectedSchemaVersion) {
      fail(`${shardPath}: schema_version must match reliability shard schema`);
    }
  }
}

function runSelfTest() {
  const schema = {
    type: "object",
    required: ["schema_version", "items"],
    properties: {
      schema_version: { const: "self-test/v1" },
      items: {
        type: "array",
        items: {
          type: "object",
          required: ["id", "nested"],
          properties: {
            id: { type: "string" },
            nested: {
              type: "object",
              required: ["value"],
              properties: {
                value: { type: "string" },
              },
            },
          },
        },
      },
    },
  };
  const errors = requiredFieldErrors({ items: [{ id: "ok", nested: {} }] }, schema, "self");
  for (const expected of [
    "self: missing required field schema_version",
    "self.items[0].nested: missing required field value",
  ]) {
    if (!errors.includes(expected)) {
      fail(`self-test did not report expected error: ${expected}`);
    }
  }
  if (requiredFieldErrors({ schema_version: "self-test/v1", items: [] }, schema, "self").length > 0) {
    fail("self-test valid sample should not report required-field errors");
  }
  console.log("operator reliability schema smoke self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

for (const contract of schemaContracts) {
  checkSchemaContract(contract);
}
checkReliabilityShards();

console.log("operator reliability schema smoke passed");
