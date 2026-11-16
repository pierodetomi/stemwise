# Stemwise name and logo

The code in this repository is open source under the [MIT licence](LICENSE). The **Stemwise name and logo are
not**: they are not part of what the MIT licence grants, and all rights in them are reserved by Piero De Tomi.

This is so that "Stemwise" always means the app published by its author, and a modified version is never mistaken
for it.

## What is reserved

- The name **Stemwise**, alone or with other words.
- The Stemwise mark (the two trajectories meeting in an exit), the lockup with the name, and the app icon, in every
  colour and size.
- The files that hold them:
  - [design/brand/](design/brand/), all of it, including the directions that were not chosen;
  - [app/src-tauri/icons/](app/src-tauri/icons/);
  - [app/ui/public/favicon.svg](app/ui/public/favicon.svg);
  - the drawing of the mark in `Mark`, in [app/ui/src/shared/Icon.tsx](app/ui/src/shared/Icon.tsx), and in
    `mark()`, in [design/tools/lib.mjs](design/tools/lib.mjs).

Everything else in the repository, the rest of the interface and of the design system included, is MIT.

## What you can do without asking

- Use the name to refer to this project: "built on Stemwise", "a fork of Stemwise", "compatible with Stemwise
  projects".
- Write, talk and post about Stemwise, with screenshots of the app and the logo unchanged, to illustrate what you
  are saying.
- Build the app from this source for your own use, and pass on unmodified builds, with the name and logo as they
  are.

## What you cannot do

- Distribute a modified version, or a product built from this code, under the name Stemwise or with its logo or
  icon. A fork that is published needs its own name and its own icon.
- Use a name, logo or icon similar enough to be confused with them.
- Use the name or logo in a way that suggests the author made, endorses or supports your product, service or
  company.
- Use the name or logo in a domain name, an account name, an app store listing or on merchandise.

## If you publish a fork

1. Give it another name: `productName` and `identifier` in
   [app/src-tauri/tauri.conf.json](app/src-tauri/tauri.conf.json), and the name where the interface shows it.
2. Replace the icons in `app/src-tauri/icons/`, the favicon, and the `Mark` drawing.
3. You may, and under the MIT licence you must, keep the copyright notice in [LICENSE](LICENSE), and you are
   welcome to say that your project is a fork of Stemwise.

For anything not covered here, ask first: open an issue or write to the author through
[pierodetomi.dev](https://pierodetomi.dev).
