# Warp No Slop

Rust terminal fork with a separate Swift prototype in `macos-swift/`.

On macOS, install the build prerequisites and package the Rust app:

```sh
./script/bootstrap --skip-common-skills
WARP_SKIP_COMMON_SKILLS_INSTALL=1 ./script/run --release --dont-open
```

The local OSS application is written to `target/release/bundle/osx/WarpOss.app`.
See `LICENSE-AGPL` and `LICENSE-MIT` for license terms.
