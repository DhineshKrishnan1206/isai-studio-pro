# Contributing to Isai Studio Pro

Thank you for helping. Isai is a free music studio for everyone, including
people on old or low-spec computers, and every contribution keeps it that way.

## How to send a change

1. **Fork** the repository on GitHub and clone your fork.
2. Make a **branch** for your change: `git checkout -b fix-fade-handles`.
3. Build, test and lint (see below), then commit.
4. Open a **pull request** to `main`. Describe what changed and how you checked
   it; add a screenshot for anything visual.

Every pull request runs the CI on Linux, Windows and macOS, and is reviewed
before it's merged. Nothing reaches `main` without a pull request.

For a bigger change (a new instrument, a new window), open an issue first so
we can agree on the approach before you spend time on it.

## Build and check

You need Rust 1.88+ and Qt 6.9+ (see the [README](README.md#build-and-run)).

```sh
cargo run --release                                    # run the app
cargo test --workspace                                 # all tests
cargo clippy --workspace --all-targets -- -D warnings  # lint, as CI does
```

The app can render any screen to an image, which helps when checking a visual
change (see "Screenshots without clicking around" in the README).

## Rules of the codebase

These keep Isai fast and consistent. The README explains each one.

1. The audio thread never allocates, frees, locks, prints or does I/O.
2. The engine crate never depends on UI code.
3. Every edit is one undo step.
4. Colours, sizes and fonts come from `Theme`; icons from `Icons`; text is `DText`.
5. Animations use `Theme.fast` / `Theme.slow`, so "animations off" is respected.
6. Every heavy feature has a low-spec option.
7. The UI only polls the engine while something moves.

[HANDOFF.md](HANDOFF.md) explains how the pieces fit together.

## Licence of contributions

Isai is licensed under the [GPL-3.0-or-later](LICENSE). By sending a pull
request you agree that your contribution is licensed under the same terms, and
that you have the right to contribute it.

- Only add code, sounds, images or fonts that you made yourself or that have a
  licence compatible with the GPL-3.0 (for example MIT, Apache-2.0, BSD,
  MPL-2.0, or the SIL Open Font License for fonts). Add anything from a third
  party to [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
- Don't name instruments, presets or features after other companies' products.

## Privacy tip

Git puts your email address in every commit. To keep it private, use the
address GitHub gives you at **Settings → Emails → Keep my email address
private**, and set it with `git config user.email`.

## Be kind

Be respectful and patient with everyone, especially people who are new to
music software or to open source.
