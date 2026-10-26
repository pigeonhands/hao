// Runs hao inside a wasm runtime: node run.mjs <hao_wasm_tests.wasm> <assembly.dll> [global-metadata.dat GameAssembly.so]
import { readFileSync } from "node:fs";

const [wasmPath, assemblyPath, metadataPath, binaryPath] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const wasm = instance.exports;

function copyIn(path) {
  const data = readFileSync(path);
  const ptr = wasm.alloc(data.length);
  new Uint8Array(wasm.memory.buffer, ptr, data.length).set(data);
  return [ptr, data.length];
}

const results = {
  rewrite_assembly: Number(wasm.rewrite_assembly(...copyIn(assemblyPath))),
  create_assembly: Number(wasm.create_assembly()),
};
if (metadataPath && binaryPath) {
  results.il2cpp_assemblies = Number(wasm.il2cpp_assemblies(...copyIn(metadataPath), ...copyIn(binaryPath)));
}
console.log(JSON.stringify(results));
