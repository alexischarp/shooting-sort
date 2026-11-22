# Shooting Sort

Desktop app to cull a day of shooting: review the JPEGs, mark each photo Keep or To Remove, then move the RAW and JPEG files into separate folders.

Rust, egui, eframe on wgpu. The app is in early development. It runs on macOS. The code has no macOS-specific parts, so Linux and Windows are candidates, but they are not tested yet.

## Features

- Opens a shoot folder that contains RAW and JPEG files together.
- Shows one photo for each RAW + JPEG pair with the same base name, for example `DSC04218.ARW` and `DSC04218.JPG`. The app shows the JPEG, because it loads faster.
- Shows the formats of each photo: `RAW + JPEG`, `JPEG only` or `RAW only`. For a RAW-only photo, the app shows the preview that the camera embeds in the RAW file.
- Marks each photo Keep or To Remove, with buttons or with the keyboard. Each marked photo shows a badge in the filmstrip.
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
- There is no `.app` bundle or installer yet. You build the app yourself.
- The app shows the photo to fit the window. It has no zoom.

## Requirements

- macOS. Linux and Windows are not tested.
- Rust 1.95 or newer, to build the app. egui 0.37 needs this version.

## Install

Build the release binary:

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
| `K` | Keep |
| `X` | To Remove |
| `U` | Clear the mark |
| `⌘O` | Open a folder |

To clear a mark with the mouse, click the active Keep or To Remove button again.

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

A pool of background threads decodes the images. The photo on screen and the photos next to it have priority. Filmstrip thumbnails load when they come into view.

The code is in `src/`:

| File | Contents |
| --- | --- |
| `library.rs` | Folder scan and RAW + JPEG pairing |
| `session.rs` | Read and write `.shooting-sort.json` |
| `sorter.rs` | File moves |
| `loader.rs` | Background image decoding |
| `theme.rs` | Colours, logo and icons |
| `app.rs` | User interface |

## Development

Run the app in debug mode:

```bash
cargo run -- /path/to/shoot-folder
```

Run the unit tests:

```bash
cargo test
```

The unit tests cover the file destinations and the sort on real files in a temporary folder. The user interface and the RAW previews need a manual test with a real shoot folder.

Run the linter:

```bash
cargo clippy
```

## Contributing

Issues and pull requests are welcome.

- Run the unit tests and `cargo clippy`. Test the app with a real shoot folder when you modify the scan, the loader or the sort.
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
