# <img src="icons/nbtworkbench.png" width=48> NBT Workbench

### [Downloads & Releases here!](https://github.com/RealRTTV/nbtworkbench/releases)

NBT Workbench is an [NBT](https://wiki.vg/NBT) editing application,
the successor to [NBT Studio](https://github.com/tryashtar/nbt-studio),
which is in turn the successor to [NBTExplorer](https://github.com/jaquadro/NBTExplorer).
NBT Workbench is written completely from scratch in [Rust](https://www.rust-lang.org/) and designed to be as performant and efficient as possible.

## <img src="icons/features.png" width=16> Features
(Features marked with a ☆ are new and not available in NBT Studio or Explorer):

* Java NBT files (`level.dat` / `hotbar.nbt`)
* Java region files (`.mca` / `.mcr`)
  * ☆ Now supports the new 1.21 LZ4 compression format
* SNBT files (`.snbt`)
* ☆ [Web Version](https://rttv.ca/main)
* Save as dialog
* Create new nbt file / new region file
* Tags can be selected, dragged and dropped to move them around.
* ☆ Action wheel
  * By holding right-click over an NBT tag: A circular action wheel will appear, which will let you make specific changes to NBT tags, this includes:
  * Copying the condensed/raw or formatted/pretty SNBT version of a tag.
  * ☆ Opening an array in a preferred hex editor.
  * ☆ Opening NBT as SNBT in a preferred text editor.
  * ☆ Sorting Compounds alphabetically or by type.
* ☆ Editing tag key/values in one click by simply being overtop the text.
* ☆ Searching with substrings, regex and snbt matching.
* ☆ Bookmarks
* ☆ Line Numbers
* ☆ Dark Mode
* ☆ Colored Text
* ☆ Remastered NBT Explorer Art
* ☆ CLI Mode `nbtworkbench -?`
  * ☆ `nbtworkbench find` to search across multiple files
  * ☆ `nbtworkbench reformat` to reformat the extensions of multiple files
* ☆ Tabs
* ☆ The fastest NBT read / write around

## <img src="icons/keybinds.png" width=16> Keybinds (in order of processing)
(Keybinds marked with a ☆ are new and not available in NBT Studio or Explorer):
* (on Selected Text)
  * \[↑\] moves up to previous line.
  * \[↓\] moves down to next line.
  * ☆ \[Ctrl + ↑\] moves up to first child with the same parent.
  * ☆ \[Ctrl + ↓\] moves down to last child with the same parent.
  * ☆ \[Ctrl + Shift + ↑\] moves element up one.
  * ☆ \[Ctrl + Shift + ↓\] moves element down one.
  * \[Alt + ←\] closes currently selected element.
  * \[Alt + →\] opens currently selected element.
  * ☆ \[Alt + Shift + →\] fully expands currently selected element.
* \[Ctrl + F\] Focus find box.
* \[+\] Zoom in.
* \[-\] Zoom out.
* ☆ \[1 to 8\] Jump to nth tab.
* ☆ \[9\] Jump to last tab.
* \[Ctrl + R\] Reload tab.
* ☆ \[Ctrl + Shift + R\] Toggle freehand mode. (Disables selecting text and makes toggle button extend horizontally to make for quick maneuvering)
* \[Ctrl + N\] New tab.
* \[Ctrl + Shift + N\] New region file tab.
* \[Ctrl + O\] Open file.
* \[Ctrl + S\] Save file.
* \[Ctrl + Shift + S\] Save file as.
* ☆ \[Ctrl + W\] Close tab.
* \[Ctrl + Z\] Undo.
* \[Ctrl + Y\] / \[Ctrl + Shift + Z\] Redo.
* ☆ \[Ctrl + D\] Duplicate hovered element below.
* \[Ctrl + C\] Copy hovered element as SNBT to clipboard.
* ☆ \[Ctrl + Shift + C\] Copy hovered element as pretty SNBT to clipboard.
* \[Ctrl + X\] Cut hovered element as SNBT to clipboard.
* ☆ (to create new template elements)
  * \[1\] Create byte.
  * \[2\] Create short.
  * \[3\] Create int.
  * \[4\] Create long.
  * \[5\] Create float.
  * \[6\] Create double.
  * \[7\] Create byte array.
  * \[8\] Create int array.
  * \[9\] Create long array.
  * \[0\] Create string.
  * \[-\] Create list.
  * \[=\] Create compound.
  * \[\`\] Create chunk.
  * \[V\] Create from clipboard.

# Credits
NBT Workbench was made by myself, Katie;
however, it would not come to be without the lovely projects below inspiring it.

### Design
* [NBT Studio by tryashtar](https://github.com/tryashtar/nbt-studio)
* [NBTExplorer by jaquadro](https://github.com/jaquadro/NBTExplorer)

### Technologies
* [WGPU](https://github.com/gfx-rs/wgpu)
* [Rust](https://rust-lang.org)

### Icons
* Remastered/Inspired by [jaquado](https://github.com/jaquadro)'s [NBTExplorer](https://github.com/jaquadro/NBTExplorer) icons.

# Compiling
### For Windows
* You must have [Rust](https://rustup.rs) 1.78.0+ \[Nightly\] (target: x86_64-pc-windows-msvc)
* Uncomment the windows-only section of your `Cargo.toml` file and make sure the other sections are commented out.
* Run the following command to make a release build in `./target/x86_64-pc-windows-msvc/release`:\
`cargo +nightly build --release --target x86_64-pc-windows-msvc -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort -- -Ctarget-feature=+avx`
### For Wasm
* You must have [Rust](https://rustup.rs) 1.78.0+ \[Nightly\] (target: x86_64-pc-windows-msvc)
* You must have [wasm-pack](https://crates.io/crates/wasm-pack) installed using cargo
* Uncomment the wasm-only section of your `Cargo.toml` file and make sure the other sections are commented out.
* Run the following command to compile for web assembly in `./web`:\
`wasm-pack build --release --target web --out-name nbtworkbench --out-dir web`
