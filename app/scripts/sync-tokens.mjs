// Copy the design tokens from docs (the single source of truth) into the app.
// Run: npm run tokens. A test fails if the copy drifts.
import { copyFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const src = fileURLToPath(new URL("../../docs/05-design/tokens.css", import.meta.url));
const dst = fileURLToPath(new URL("../src/styles/tokens.css", import.meta.url));
copyFileSync(src, dst);
console.log(`tokens.css copied to ${dst}`);
