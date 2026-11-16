# Security

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub: on the repository page, **Security › Report a vulnerability**
([direct link](https://github.com/pierodetomi/stemwise/security/advisories/new)). Say what you found, how to
reproduce it and what an attacker could do with it.

Stemwise is maintained by one person in his spare time, so there is no guaranteed response time. Reports are read,
taken seriously and answered as soon as time allows; a fix is published in a new release, with credit to you if
you wish.

## Supported versions

Only the latest release receives fixes.

## What is in scope

Stemwise is a desktop app that works on local files. The parts that matter most for security are:

- **The OpenRouter key.** It is stored on your Mac, in the system keychain when the build is signed, otherwise in
  a file private to the app. It is sent only to OpenRouter.
- **Network.** The app talks to OpenRouter only, and only when you ask for something from the assistant or connect
  your account. There is no telemetry and no update check.
- **What is sent to the model.** Measurements, track roles and names, your brief and your request. Never audio,
  file names, paths or the song's title. A way to make the app send more than that is a vulnerability.
- **Opening files.** Audio files, project packages and cover images are parsed by the app: a file that makes it
  crash, hang or write outside the project is worth reporting.

## What is not

- The security of OpenRouter or of the model providers: report those to them.
- The fact that the app is not signed with an Apple Developer ID. It is a known limit, stated in the README.
