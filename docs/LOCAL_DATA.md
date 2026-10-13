# Local data and permissions

Code Atlas keeps repository captures and guide history on this Mac. This page
describes the current source behavior; the project does not yet ship a signed,
notarized installer.

## Where Code Atlas stores data

The Rust app asks Tauri for its application data directory. With the current
bundle identifier, `com.codeatlas.desktop`, the default macOS location is:

~~~text
~/Library/Application Support/com.codeatlas.desktop/
~~~

That location follows Tauri's [`app_data_dir`](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html#method.app_data_dir) default: macOS Application Support plus the configured bundle identifier. The app does not override the directory.

The app stores recent-project records, captured snapshots and source objects,
indexes, chapter and PR Guide revisions, recovery records, and cached narration
audio under that application data directory. The original Git working copy
stays where you chose it; capture creates a separate local snapshot.

## First-run permissions and optional model APIs

Opening a local repository uses the native macOS folder picker. Code Atlas checks
the selected folder and its Git metadata first; source bytes are read only when
you choose **Create snapshot**. Local browsing, snapshots, indexes, and saved
guides work without an AI key.

Direct model APIs are optional. If you add a provider key in **AI and
narration setup**, Code Atlas saves it in a provider-specific macOS Keychain
item and never writes it to an app data file. Gemini narration uses a separate
Gemini key. macOS may ask you to unlock Keychain or approve access. If access
is denied, the app says the key was not saved; retry after unlocking Keychain
or approving the access request. Local browsing remains available. You can
remove a key from the same settings screen.

## Exports and backups

**Offline export** creates one HTML guide from the accepted revisions you
select. It can include their saved narration transcripts and audio that is
already cached on this Mac. It does not export repository snapshots, indexes,
draft history, recovery records, or recent-project metadata.

There is no complete application backup, restore, or import workflow yet. Keep
the original Git repository as the canonical source. Treat exported HTML as a
reader copy of the selected guide, not as a restorable Code Atlas project.

## Removing local data

**Remove from Recents** removes a project path from the recent-project list. It
does not delete that project's snapshots, source objects, indexes, or guide
history.

To remove all Code Atlas data from this Mac, quit the app, open Finder's **Go >
Go to Folder…**, enter `~/Library/Application Support/`, and move the
`com.codeatlas.desktop` folder to the Trash. This removes local data for every
project in Code Atlas. Emptying the Trash makes that deletion permanent. An
offline HTML export stored elsewhere is not removed. Direct API keys are kept
in provider-specific Keychain entries, and Gemini narration has its own key;
remove the relevant keys in **AI and narration setup** if you also want to
delete them.

## If storage is low

Snapshots, guide revisions, and narration audio use local disk space. If a
capture or save fails while the Mac is low on space, free space outside the
Code Atlas data folder, reopen the app, and retry. Do not delete the application
data folder as a space-saving step unless you intend to remove all local
projects and their saved history.

The exact install, supported macOS versions, signing, notarization, and clean
account behavior remain release gates. See the [macOS packaging task](kata-tasks/25-macos-packaging.md).
