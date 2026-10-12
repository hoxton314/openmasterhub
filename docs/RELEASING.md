# Releasing

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed, and go to the
AUR as [`openmasterhub-bin`](https://aur.archlinux.org/packages/openmasterhub-bin) only after a
manual approval.

## Cutting a release

1. Bump `version` in the root `Cargo.toml` (`[workspace.package]`), run `cargo build` so
   `Cargo.lock` follows, and merge that to `main`.
2. Tag and push:
   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```
3. The **build** job runs on Arch Linux:
   - It checks that the tag matches the Cargo version.
   - It runs clippy and the tests.
   - It builds the release binaries and stages `openmasterhub-<ver>-x86_64.tar.gz`.
   - It renders the AUR `PKGBUILD` with the real sha256 and **builds the package with makepkg**.
   - It uploads the tarball to a **draft** GitHub release.
4. Check the draft release, then open the workflow run in *Actions*. The **publish** job waits
   for approval of the `aur` environment, so click **Review deployments → Approve**. It then:
   - publishes the GitHub release
   - verifies the public tarball against the PKGBUILD checksum
   - pushes `PKGBUILD`, `.SRCINFO` and `openmasterhub.install` to the AUR

If the build fails, fix it, delete the tag and draft release, and tag again. Nothing reaches
the AUR or the public release page before the approval.

## One-time setup

- Environment `aur` (Settings → Environments) with **Required reviewers** set to the
  maintainer and a deployment rule allowing only `v*` tags. This already exists. Without the
  protection rule the publish job would run without waiting.
- AUR credentials as **secrets of the `aur` environment**, so only the approved job can read
  them (the same values as git-twig's repository secrets):
  ```sh
  gh secret set AUR_USERNAME        --env aur -R hoxton314/openmasterhub
  gh secret set AUR_EMAIL           --env aur -R hoxton314/openmasterhub
  gh secret set AUR_SSH_PRIVATE_KEY --env aur -R hoxton314/openmasterhub < ~/.ssh/aur
  ```
  The key's public half must be registered in the AUR account. The first push creates the
  `openmasterhub-bin` package.

## Testing the packaging locally (Arch)

```sh
cargo build --release
.github/scripts/stage-release.sh 0.1.0 target /tmp/release
.github/scripts/test-pkgbuild.sh /tmp/release 0.1.0
```
