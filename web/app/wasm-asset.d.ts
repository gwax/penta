// Vite serves `?url` imports as a plain URL string. wasm-bindgen's own loader
// would otherwise fall back to `new URL(..., import.meta.url)`, which Vite 8
// resolves to a file: URL that the browser refuses to load.
declare module "*.wasm?url" {
  const url: string;
  export default url;
}

// Workers import a .wasm file as a compiled module rather than fetching it.
declare module "*.wasm" {
  const compiled: WebAssembly.Module;
  export default compiled;
}
