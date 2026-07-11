#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const datasetSchemaPath = "schemas/workflow-dataset.schema.json";
const graphSchemaPath = "schemas/workflow-graph.schema.json";
const datasetExamplePath = "schemas/examples.workflow-dataset.json";
const graphExamplePath = "schemas/examples.workflow-graph.json";
const docsPath = "docs/workflow-dataset.md";

const DATASET_SCHEMA_VERSION = "kyuubiki.workflow-dataset/v1";
const GRAPH_SCHEMA_VERSION = "kyuubiki.workflow-graph/v1";
const REQUIRED_DATA_CLASSES = [
  "study_model",
  "result",
  "field",
  "table",
  "report",
  "export",
  "scalar",
  "metadata",
];

function fail(message) {
  console.error(`workflow dataset contract check failed: ${message}`);
  process.exit(1);
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function requireString(value, field, context) {
  if (typeof value !== "string" || value.trim() === "") {
    fail(`${context}: ${field} must be a non-empty string`);
  }
}

function checkSchema(schema) {
  if (schema?.properties?.schema_version?.const !== DATASET_SCHEMA_VERSION) {
    fail(`${datasetSchemaPath}: schema_version const must be ${DATASET_SCHEMA_VERSION}`);
  }
  const enumValues = dataClassesFromSchema(schema);
  for (const dataClass of REQUIRED_DATA_CLASSES) {
    if (!enumValues.includes(dataClass)) {
      fail(`${datasetSchemaPath}: data_class enum is missing ${dataClass}`);
    }
  }
  if (schema?.$defs?.valueInfo?.properties?.element_type?.minLength !== 1) {
    fail(`${datasetSchemaPath}: element_type must require minLength 1`);
  }
  if (schema?.$defs?.axis?.properties?.id?.minLength !== 1) {
    fail(`${datasetSchemaPath}: axis.id must require minLength 1`);
  }
  if (schema?.$defs?.schemaRef?.properties?.schema?.minLength !== 1) {
    fail(`${datasetSchemaPath}: schema_ref.schema must require minLength 1`);
  }
}

function dataClassesFromSchema(schema) {
  const enumValues = schema?.$defs?.valueInfo?.properties?.data_class?.enum;
  if (!Array.isArray(enumValues) || enumValues.length === 0) {
    fail(`${datasetSchemaPath}: data_class enum must be a non-empty array`);
  }
  return enumValues;
}

function checkGraphSchema(schema) {
  if (schema?.properties?.schema_version?.const !== GRAPH_SCHEMA_VERSION) {
    fail(`${graphSchemaPath}: schema_version const must be ${GRAPH_SCHEMA_VERSION}`);
  }
  if (schema?.properties?.dataset_contract?.$ref !== "workflow-dataset.schema.json") {
    fail(`${graphSchemaPath}: dataset_contract must reference workflow-dataset.schema.json`);
  }
}

function checkDatasetContract(contract, context, allowedDataClasses) {
  if (contract.schema_version !== DATASET_SCHEMA_VERSION) {
    fail(`${context}: schema_version must be ${DATASET_SCHEMA_VERSION}`);
  }
  requireString(contract.id, "id", context);
  requireString(contract.version, "version", context);
  if (!Array.isArray(contract.values) || contract.values.length === 0) {
    fail(`${context}: values must be a non-empty array`);
  }

  const valueIds = new Set();
  for (const [index, value] of contract.values.entries()) {
    const valueContext = `${context}#values/${index}`;
    requireString(value.id, "id", valueContext);
    if (valueIds.has(value.id)) {
      fail(`${context}: duplicate dataset value id ${value.id}`);
    }
    valueIds.add(value.id);
    if (!allowedDataClasses.includes(value.data_class)) {
      fail(`${valueContext}: unsupported data_class ${value.data_class}`);
    }
    requireString(value.element_type, "element_type", valueContext);
    if (value.semantic_type !== undefined) {
      requireString(value.semantic_type, "semantic_type", valueContext);
    }
    if (value.unit !== undefined) {
      requireString(value.unit, "unit", valueContext);
    }
    checkShape(value.shape ?? {}, valueContext);
    if (value.schema_ref !== undefined) {
      requireString(value.schema_ref?.schema, "schema_ref.schema", valueContext);
      requireString(value.schema_ref?.version, "schema_ref.version", valueContext);
    }
  }
  return new Map(contract.values.map((value) => [value.id, value]));
}

function checkShape(shape, context) {
  const axes = Array.isArray(shape.axes) ? shape.axes : [];
  const axisIds = new Set();
  for (const [index, axis] of axes.entries()) {
    requireString(axis.id, `shape.axes/${index}.id`, context);
    if (axisIds.has(axis.id)) {
      fail(`${context}: duplicate shape axis id ${axis.id}`);
    }
    axisIds.add(axis.id);
  }
}

function checkGraphExample(graph, allowedDataClasses) {
  if (graph.schema_version !== GRAPH_SCHEMA_VERSION) {
    fail(`${graphExamplePath}: schema_version must be ${GRAPH_SCHEMA_VERSION}`);
  }
  const values = checkDatasetContract(
    graph.dataset_contract,
    `${graphExamplePath}#dataset_contract`,
    allowedDataClasses,
  );
  const nodeById = new Map((graph.nodes ?? []).map((node) => [node.id, node]));

  for (const node of graph.nodes ?? []) {
    for (const port of [...(node.inputs ?? []), ...(node.outputs ?? [])]) {
      if (!port.dataset_value) {
        continue;
      }
      const value = values.get(port.dataset_value);
      if (!value) {
        fail(`${graphExamplePath}: port ${node.id}.${port.id} references unknown dataset value ${port.dataset_value}`);
      }
      if (value.semantic_type && value.semantic_type !== port.artifact_type) {
        fail(`${graphExamplePath}: port ${node.id}.${port.id} artifact_type does not match dataset semantic_type`);
      }
    }
  }

  for (const edge of graph.edges ?? []) {
    const fromPort = nodeById.get(edge.from?.node)?.outputs?.find((port) => port.id === edge.from?.port);
    const toPort = nodeById.get(edge.to?.node)?.inputs?.find((port) => port.id === edge.to?.port);
    if (!fromPort || !toPort) {
      fail(`${graphExamplePath}: edge ${edge.id} references missing port`);
    }
    const datasetValue = edge.dataset_value ?? fromPort.dataset_value ?? toPort.dataset_value;
    const value = values.get(datasetValue);
    if (!value) {
      fail(`${graphExamplePath}: edge ${edge.id} references unknown dataset value ${datasetValue}`);
    }
    if (fromPort.dataset_value && fromPort.dataset_value !== datasetValue) {
      fail(`${graphExamplePath}: edge ${edge.id} disagrees with source port dataset value`);
    }
    if (toPort.dataset_value && toPort.dataset_value !== datasetValue) {
      fail(`${graphExamplePath}: edge ${edge.id} disagrees with target port dataset value`);
    }
    if (value.semantic_type && value.semantic_type !== edge.artifact_type) {
      fail(`${graphExamplePath}: edge ${edge.id} artifact_type does not match dataset semantic_type`);
    }
  }
}

function checkDocumentation() {
  const docs = readText(docsPath);
  for (const phrase of [
    "dataset value ids inside one contract must be non-empty and unique",
    "`data_class` must stay inside the stable workflow dataset class set",
    "shape axis ids must be unique inside each dataset value",
  ]) {
    if (!docs.includes(phrase)) {
      fail(`${docsPath}: missing runtime rule "${phrase}"`);
    }
  }
}

function checkContracts() {
  const datasetSchema = readJson(datasetSchemaPath);
  checkSchema(datasetSchema);
  checkGraphSchema(readJson(graphSchemaPath));
  const allowedDataClasses = dataClassesFromSchema(datasetSchema);
  checkDatasetContract(readJson(datasetExamplePath), datasetExamplePath, allowedDataClasses);
  checkGraphExample(readJson(graphExamplePath), allowedDataClasses);
  checkDocumentation();
  console.log("workflow dataset contract check passed");
}

function runSelfTest() {
  const example = readJson(datasetExamplePath);
  example.values.push({ ...example.values[0] });
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    checkDatasetContract(example, datasetExamplePath, dataClassesFromSchema(readJson(datasetSchemaPath)));
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject duplicate dataset value id");
  }
  console.log("workflow dataset contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
