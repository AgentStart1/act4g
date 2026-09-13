# act4g

act4g is a focused Android client for keeping up with the GitHub activity that
matters to you.

> act4g is in early development and is not an official GitHub application.

## Features

- View GitHub notifications and distinguish unread items
- Search by repository name or notification title
- Read details for issues, pull requests, releases, and check runs
- Open the corresponding page on GitHub from a notification
- Keep your session securely between app launches
- Use the app comfortably on full-screen, cutout, landscape, and split-screen
  layouts

## Download and installation

act4g has not been released yet. When builds become available, they will be
published on the [Releases](https://github.com/storytellerF/act4g/releases)
page.

- **Release** builds are intended for everyday use.
- **Alpha** builds contain the newest changes and may be less stable. Their app
  icon includes an `ALPHA` ribbon.

act4g supports Android 8.0 (API 26) and newer. Android may ask you to allow your
browser or file manager to install unknown apps before installing an APK.

The Android package name is `com.storytellerf.act4g`.

## Sign in to GitHub

1. Open act4g and tap **Sign in with GitHub**.
2. act4g opens GitHub's device authorization page in your browser.
3. Confirm the displayed code and approve access.
4. Return to act4g. The app completes sign-in and loads your notifications.

act4g never asks you to enter your GitHub password inside the app. Your session
credential is stored in Android's secure storage and removed when you sign out.

## Using the app

- Tap **Refresh** to load the latest notifications.
- Tap a notification card to view its details.
- After the details load, tap **Open on GitHub** to open the matching web page.
- If a request fails, confirm that your device can reach GitHub and check your
  system proxy settings.

## Frequently asked questions

### Why are there no notifications after I sign in?

Confirm that the GitHub account has notifications, then tap **Refresh** to try
again.

### Why is Open on GitHub temporarily unavailable?

Some notification types require an additional request to resolve the exact
GitHub URL. The button remains unavailable while that request is loading or if
it fails, preventing the app from opening the wrong page.

## Privacy and permissions

act4g uses network access to communicate with the GitHub API and uses your
system browser for GitHub authorization. The current version has no independent
account server and does not collect your GitHub password.

You can sign out inside the app at any time or clear its data from Android
settings.

## Report a problem

If notifications do not load, a screen looks incorrect, or a link opens the
wrong page, please open an [issue](https://github.com/storytellerF/act4g/issues)
and include:

- Your Android version and device model
- The act4g build type, if known
- Steps that reproduce the problem
- Any screenshots or error messages that are safe to share

Do not include access tokens, authorization codes, or other sensitive data.

## Development

act4g is built with Rust, GPUI Mobile, and Android Gradle. Pull requests run
Rust tests, Clippy, Detekt, and an Android debug build.

The project is under active development. Issues and pull requests are welcome.
