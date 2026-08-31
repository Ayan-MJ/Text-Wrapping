import { copyFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(
  new URL("../../target/wasm32-unknown-unknown/release/lastdraft_flow.wasm", import.meta.url),
);
const outputDirectory = fileURLToPath(new URL("../dist", import.meta.url));
const output = fileURLToPath(new URL("../dist/lastdraft_flow.wasm", import.meta.url));

await mkdir(outputDirectory, { recursive: true });
await copyFile(source, output);

