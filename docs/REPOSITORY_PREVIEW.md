# Try the real repository preview

The ordinary build includes OmaCalc, OmaCut, OmaWrite, OmaPresent, LocalSend and Heroic Games Launcher. Their package identities were observed in Omarchy's stable x86_64 index. These are community entries with upstream attribution, not claimed author accounts. Use the ordinary app without `--demo`; sample mode still shows fictional apps and transactions.

The earlier, pre-polish package is available from the artifacts section of [Arch build 34398115220](https://github.com/tcballard/OmaStore/actions/runs/34398115220), built from commit `94214915628ba532dc0d7262c442534785c1dbb7`. Download `omastore-preview-arch-x86_64`, extract it, and run `sha256sum -c SHA256SUMS` before using the package installation command below. This has passed the Arch build and content checks; actual Omarchy desktop acceptance remains pending.

## Build an installable package on Omarchy

Update Omarchy through its normal updater first. Install the build prerequisites listed in the README, then run from the repository checkout:

```sh
rustup show
./bin/build-repository-catalogue --check
cd packaging
makepkg --cleanbuild
python check-package.py ./omastore-preview-0.1.0-3-x86_64.pkg.tar.zst
sudo pacman -U ./omastore-preview-0.1.0-3-x86_64.pkg.tar.zst
omastore
```

The package installs only OmaStore's desktop binaries, launcher entry, metadata and licence. It does not install catalogue apps, launch a server or enable payments. The package revision is specific to this preview; inspect the filename if working on a newer revision. Updating a local build with the same package version still requires explicitly reinstalling it.

The **Arch preview package** GitHub workflow builds the same recipe as an unprivileged user and uploads an artifact containing the package, `SHA256SUMS` and `SOURCE_COMMIT`. It does not publish a release or add OmaStore to Omarchy's repository. A successful run is required before claiming that artifact is available. Verify the commit and checksum before installing a downloaded build.

## First desktop exercise

1. Open OmaStore through the application launcher. Record the application commit/package version and Omarchy version.
2. Find OmaCalc using search, open its details and save it. Confirm package `omarchy/omacalc`, the observed version, unknown maturity and the absence of an OmaStore test claim.
3. Open its Source link. Close and reopen OmaStore and check the saved entry. Test offline browsing as well.
4. Choose **Review installation plan**. Record the detected host and any blockers. A missing distribution service or unverified live adapter must remain a blocker; this exercise does not authorise bypassing it.
5. Resize the window and exercise keyboard navigation, light/dark appearance and your normal display scaling. Share screenshots and any failures.

The next installation milestone needs a disposable, accessible Omarchy host for the actual package lifecycle. The current package preview supports real discovery and read-only planning, while managed installation remains disabled. Browsing does not depend on Stripe or recruiting authors. Existing cached catalogue content can take precedence over the bundle; report a stale/empty view instead of deleting your application state.

Ordinary author submissions still use independent review and approved publication. This initial package-index selection is documented in ADR 0010. Updating an index observation does not establish an application test, signature verification or publisher consent to reuse screenshots.
