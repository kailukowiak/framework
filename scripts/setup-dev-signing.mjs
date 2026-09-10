import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";

const identity = "FrameWork Local Development";

function run(command, args, options = {}) {
  return execFileSync(command, args, { stdio: "inherit", ...options });
}

if (process.platform !== "darwin") {
  console.log("Local development signing is only needed on macOS.");
  process.exit(0);
}

const existing = execFileSync("security", ["find-identity", "-v", "-p", "codesigning"], {
  encoding: "utf8",
});
const hasExistingIdentity = existing
  .split("\n")
  .some((line) => line.includes(`\"${identity}\"`) && !line.includes("CSSMERR_"));
if (hasExistingIdentity) {
  console.log(`The ${identity} identity is already installed.`);
  process.exit(0);
}

const keychain = execFileSync("security", ["default-keychain", "-d", "user"], {
  encoding: "utf8",
}).trim().replace(/^\"|\"$/g, "");
const directory = mkdtempSync(join(tmpdir(), "framework-dev-signing-"));
const config = join(directory, "certificate.conf");
const key = join(directory, "identity.key");
const certificate = join(directory, "identity.pem");
const archive = join(directory, "identity.p12");
const password = randomUUID();

writeFileSync(
  config,
  `[req]
prompt = no
distinguished_name = subject
x509_extensions = extensions

[subject]
CN = ${identity}

[extensions]
basicConstraints = critical, CA:true
keyUsage = critical, digitalSignature, keyCertSign
extendedKeyUsage = codeSigning
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always
`,
);

try {
  run("openssl", [
    "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-sha256",
    "-days", "3650", "-config", config, "-keyout", key, "-out", certificate,
  ]);
  run("openssl", [
    "pkcs12", "-export", "-name", identity, "-inkey", key, "-in", certificate,
    "-out", archive, "-passout", `pass:${password}`,
  ]);
  run("security", [
    "import", archive, "-k", keychain, "-f", "pkcs12", "-P", password,
    "-x", "-T", "/usr/bin/codesign", "-T", "/usr/bin/security",
  ]);
  run("security", [
    "add-trusted-cert", "-r", "trustRoot", "-p", "codeSign", "-k", keychain,
    certificate,
  ]);

  const installed = execFileSync("security", ["find-identity", "-v", "-p", "codesigning"], {
    encoding: "utf8",
  });
  const hasInstalledIdentity = installed
    .split("\n")
    .some((line) => line.includes(`\"${identity}\"`) && !line.includes("CSSMERR_"));
  if (!hasInstalledIdentity) {
    throw new Error(`Installed ${identity}, but macOS does not report it as a valid signing identity.`);
  }
  console.log(`Installed ${identity}. Future FrameWork Dev rebuilds will retain macOS permissions.`);
} finally {
  // The only lasting copy of the private key belongs in the login keychain.
  // Do not leave an exportable key or its temporary transport password on disk.
  rmSync(directory, { recursive: true, force: true });
}
