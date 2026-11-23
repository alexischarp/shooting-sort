# Shooting Sort

Desktop app to cull a day of shooting: review the JPEGs, mark each photo Keep or To Remove, then move the RAW and JPEG files into separate folders.

Rust, egui, eframe on wgpu. The app is in early development. It is tested on macOS. Builds for Windows and Linux are available, but they are not tested yet.

## Features

- Opens a shoot folder that contains RAW and JPEG files together.
- Shows one photo for each RAW + JPEG pair with the same base name, for example `DSC04218.ARW` and `DSC04218.JPG`. The app shows the JPEG, because it loads faster.
- Shows the formats of each photo: `RAW + JPEG`, `JPEG only` or `RAW only`. For a RAW-only photo, the app shows the preview that the camera embeds in the RAW file.
- Marks each photo Keep or To Remove, with buttons or with the keyboard. Each marked photo shows a badge in the filmstrip and in the gallery.
- Shows the photos of the list in a gallery grid. You can mark the photos in the gallery too.
- Zooms to 100 % to check the focus, and up to 400 %. The zoom stays when you go to the next photo, so you can compare the photos of a burst at the same place.
- Filters the list: All Photos, Unsorted, Keep, To Remove. The sidebar shows the progress.
- Saves the marks in the shoot folder. When you open the folder again, the app restores the marks and goes to the first unsorted photo.
- Moves the files when you click "Sort Files". It never deletes a file and never overwrites a file.

## Supported formats

| Type | Extensions |
| --- | --- |
| JPEG | `jpg`, `jpeg` |
| RAW | `3fr`, `arw`, `cr2`, `cr3`, `crw`, `dcr`, `dng`, `erf`, `iiq`, `kdc`, `mef`, `mos`, `nef`, `nrw`, `orf`, `pef`, `raf`, `rw2`, `rwl`, `sr2`, `srf`, `srw` |

The extensions are not case-sensitive. The preview of a RAW-only photo comes from the [rawler](https://crates.io/crates/rawler) crate. If rawler cannot read a RAW file, the app shows an error for that photo, but you can still sort it.

## Limitations

- The app does not move XMP sidecar files. A sidecar stays in the shoot folder.
- The builds are not signed. macOS and Windows show a warning the first time you open the app: see [Install](#install).
- For a RAW-only photo, the zoom shows the embedded preview, not the RAW data. On many cameras this preview is small (about 1600 pixels wide on the Sony A7R IV).
- The full-size image is limited to the largest texture of the graphics card (often 8192 or 16384 pixels). A larger photo is shown slightly below its real 100 %.

## Requirements

- macOS 11 or newer, on Apple Silicon or Intel.
- Windows 10 or newer, 64-bit. Not tested yet.
- Linux, x86-64, with Wayland or X11 and a graphics driver with Vulkan or OpenGL. Not tested yet.
- Rust 1.95 or newer, only to build the app yourself. egui 0.37 needs this version.

## Install

Download the file for your system from the [latest release](https://github.com/alexischarp/shooting-sort/releases/latest).

### macOS

1. Open `Shooting-Sort-<version>-macos.dmg` and drag "Shooting Sort" to "Applications".
2. Open the app. macOS shows that it cannot verify the developer, because the app is not signed. Click "Done".
3. Open System Settings → Privacy & Security. Below "Security", click "Open Anyway" for Shooting Sort, then confirm.

You do this one time for each new version. As an alternative, remove the quarantine flag in the Terminal:

```bash
xattr -dr com.apple.quarantine "/Applications/Shooting Sort.app"
```

### Windows

1. Extract `Shooting-Sort-<version>-windows-x86_64.zip`.
2. Open `Shooting Sort.exe`. If Windows SmartScreen shows "Windows protected your PC", click "More info", then "Run anyway".

### Linux

Extract `Shooting-Sort-<version>-linux-x86_64.tar.gz` and run `shooting-sort`. The folder picker uses the desktop's file dialog, through the XDG desktop portal.

### Build from source

```bash
cargo build --release
```

The binary is `target/release/shooting-sort`.

## Usage

1. Start the app. To open a folder directly, give its path:

   ```bash
   cargo run --release -- /path/to/shoot-folder
   ```

2. If you did not give a path, click "Open Folder" (⌘O) or drop the folder on the window.
3. Mark each photo Keep or To Remove. After a mark, the app goes to the next photo.
4. Click "Sort Files". The dialog shows which files move where. Click "Sort Files" again to move them.
5. Import `sorted/raw` into Lightroom.

The shortcuts are:

| Key | Action |
| --- | --- |
| `←` / `→` | Previous / next photo |
| `↑` / `↓` | Photo in the row above / below, in the gallery |
| `K` | Keep |
| `X` | To Remove |
| `U` | Clear the mark |
| `G` | Gallery, or back to the photo |
| `Enter` | Open the selected photo, in the gallery |
| `Z` | Zoom to 100 %, or back to fit |
| `+` / `-` | Zoom in / out |
| `Esc` | Back to fit |
| `⌘O` | Open a folder |

To clear a mark with the mouse, click the active Keep or To Remove button again.

To zoom with the mouse or the trackpad:

- Double-click the photo to zoom to 100 % at that point. Double-click again to go back to fit.
- Pinch on the trackpad, or scroll with `⌘` held, to zoom at the pointer.
- When the photo is zoomed, drag it or scroll to move it.

The label at the bottom left of the photo shows the zoom level.

To see the photos as a grid, click "Gallery" (`G`). The arrows select a photo, and `K`, `X` and `U` mark it. Click a photo or press `Enter` to open it.

## How it works

The app reads only the top level of the shoot folder. It ignores subfolders and hidden files, for example the `._*` files that macOS writes on SD cards.

After the sort, the shoot folder looks like this:

```
2026-11-22/
├── sorted/
│   ├── raw/        RAW files of the kept photos
│   └── jpeg/       JPEG files of the kept photos
├── to-remove/      RAW and JPEG files of the removed photos
└── …               photos with no mark stay here
```

The app does not delete the files in `to-remove/`. Examine them and delete them yourself.

If a file with the same name is already at the destination, the app does not move the file. The dialog after the sort lists each file that did not move. That file and its mark stay in the shoot folder.

The marks are in `.shooting-sort.json`, in the shoot folder. The app writes this file after each mark and deletes it when no marks are left.

A pool of background threads decodes the images. The photo on screen and the photos next to it have priority. The thumbnails of the filmstrip and the gallery load when they come into view.

The main view shows a preview of 2560 pixels at most. When you zoom, the app also decodes the full-size JPEG of the current photo and of the next photo. It shows the full-size image only when the preview does not have sufficient pixels for the zoom level.

The code is in `src/`:

| File | Contents |
| --- | --- |
| `library.rs` | Folder scan and RAW + JPEG pairing |
| `session.rs` | Read and write `.shooting-sort.json` |
| `sorter.rs` | File moves |
| `loader.rs` | Background image decoding |
| `zoom.rs` | Zoom, pan and fit calculations |
| `platform.rs` | File names and opening a folder in the file manager |
| `ui/` | Colours, logo, icons and widgets, independent of the app state |
| `app.rs` | App state, actions and window layout |
| `app/culling.rs` | Photo list: filter, current photo, navigation and decisions |
| `app/images.rs` | Textures of the thumbnails, previews and full-size images |
| `app/input.rs` | Keyboard, drag-and-drop and zoom gestures |
| `app/views/` | Panels and dialogs of the window |

The app icon is `assets/icon.svg`. The files `assets/icon.icns` (macOS bundle), `assets/icon.png` and `assets/icon-128.rgba` (window icon on Windows and Linux) come from it. The macOS bundle files are in `packaging/macos/`.

## Development

Run the app in debug mode:

```bash
cargo run -- /path/to/shoot-folder
```

Run the unit tests:

```bash
cargo test
```

The unit tests cover the file destinations, the sort on real files in a temporary folder, and the zoom and pan calculations. The user interface and the RAW previews need a manual test with a real shoot folder.

Run the linter:

```bash
cargo clippy
```

Format the code:

```bash
cargo fmt
```

Enable the pre-push hook once per clone. It stops the push when the code is not formatted:

```bash
git config core.hooksPath .githooks
```

The workflow `.github/workflows/check.yml` checks the formatting, runs `cargo clippy` and the unit tests on every push to `main` and on every pull request.

## Release

The workflow `.github/workflows/release.yml` builds the macOS, Windows and Linux files, and publishes them as a GitHub release.

1. Set the new version in `Cargo.toml`, then run `cargo build` to update `Cargo.lock`. Commit the two files.
2. Tag the commit with the same version, prefixed with `v`, and push the tag:

   ```bash
   git tag v0.2.0
   ```

   ```bash
   git push origin v0.2.0
   ```

The workflow stops if the tag and the version in `Cargo.toml` are different. To test the builds without a release, run the workflow by hand from the Actions tab. The files are then in the workflow artifacts.

To make the macOS app on your computer (Apple Silicon only):

```bash
cargo build --release
```

```bash
packaging/macos/bundle.sh target/release/shooting-sort 0.1.0 dist
```

## Contributing

Issues and pull requests are welcome.

- Run `cargo fmt`, the unit tests and `cargo clippy`. Test the app with a real shoot folder when you modify the scan, the loader, the zoom or the sort.
- Write commit messages in the Conventional Commits format (`feat:`, `fix:`, `docs:`).
- Reports about RAW formats that do not show a preview are welcome. Give the camera model and the file extension.

## License

The license of Shooting Sort is the [GNU General Public License, version 3 or later](LICENSE) (GPL-3.0-or-later). In short:

- You can use, modify and share the app and its source code, also for commercial purposes.
- If you share the app or a modified version, you must share its full source code under this same license.
- A modified version must keep the copyright notices. It must also carry a prominent notice with the modification and its date.

This notice applies to every file in this repository:

```
Shooting Sort - sorts the RAW and JPEG files of a photo shoot.
Copyright (C) 2026  Alexis Charp

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
