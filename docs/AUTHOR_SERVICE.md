# Author service

The desktop opens without this service. Public browsing uses the Git catalogue; authenticated author/reviewer commands use the separately configured HTTPS origin.

Build the `omastore-service` and `omastore-admin` Cargo binaries. Run the service as a dedicated unprivileged account, with its database in a private directory and an HTTPS reverse proxy forwarding to loopback:

```sh
cargo build --release --locked --bin omastore-service --bin omastore-admin
./target/release/omastore-service data/registry.json 127.0.0.1:8080 --workspace /var/lib/omastore/private/workflow.db
```

The operator supplies `OMASTORE_SERVICE_ORIGIN` as an HTTPS origin without a path, plus `OMASTORE_GITHUB_CLIENT_ID` and `OMASTORE_GITHUB_CLIENT_SECRET` in a private service environment file. Register the exact callback `https://YOUR-ORIGIN/api/v1/auth/callback`; disable wildcard callback matching. The GitHub OAuth application requests `read:user`, not private-repository write access. Product credentials are unrelated to the coding agent's GitHub connection.

Set the same `OMASTORE_SERVICE_ORIGIN` for the desktop process. The core permits HTTPS, disables redirects and stores only OmaStore's session in Secret Service using `secret-tool` (Arch package `libsecret`). GitHub provider tokens never reach the Qt pipe. If the keyring is unavailable, the native workspace explicitly reports session-only sign-in. No plaintext credential fallback exists.

After the first actual sign-in, list account IDs and grant narrowly scoped operating roles through the local operator CLI:

```sh
./target/release/omastore-admin users /var/lib/omastore/private/workflow.db
./target/release/omastore-admin grant /var/lib/omastore/private/workflow.db github:ACTUAL-ID reviewer
```

Roles are `author`, `reviewer`, `maintainer`, `operator` and `editor`. The CLI also accepts `revoke`. It accesses the private database under operating-system file permissions; there is no public bootstrap or unauthenticated role endpoint. Revocation takes effect on the next privileged action. Reauthentication does not restore removed roles.

Private API requests require a bearer session and `X-OmaStore-Client: native-v1`. No cookies or CORS permissions authenticate these commands. Unrecognised browser origins are rejected. Requests have bounded headers, bodies, concurrency and deadlines; sign-in and claim creation have rate limits. Proxy limits should be equal or stricter. Do not expose the loopback listener directly.

The `demo` build also has a private local sample workspace in the native core, with explicitly fictional author/reviewer/operator roles. It uses a distinct development database and never configures real publication or payment credentials. To exercise the same HTTP surface, a development-feature service accepts `--workspace PRIVATE-SAMPLE.db --sandbox`; production builds reject this switch. Do not proxy a sandbox listener to the Internet. Database environment markers prevent opening a development database as a production workspace.

Real OAuth callbacks, an unlocked desktop keyring and the deployed reverse proxy need their own integration receipts. The local sample path proves the native workflow without substituting for those receipts.
