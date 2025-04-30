# OLED Mask

When a static image is left on an OLED screen for an extended period, it can result in a permanent mark in the display.
Even after the image changes to something new, burn-in leaves a faint impression of the static image.

This tool creates a black mask above static content to prevent OLED from burn-in.

![screenshot](/docs/screenshot.png)

## Usage

1. Download [labelImg](https://github.com/HumanSignal/labelImg/releases/latest).
2. Press the <kbd>PrtSc</kbd> keyboard shortcut to take a screenshot and save it as an image file.
3. Open the image file in labelImg and choose the area you want to mask. You can use the same name in different areas.
   When you are done, save the file as `annotation.xml`.
4. Place `annotation.xml` into the same directory as the program.
5. Run the program.

## Build

### Install dependencies

- Rust
- Visual Studio

### Setup environment

Add `vcvarsall.bat` to the `PATH` environment variable. Usually the directory is `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build`.

### Compile

Run `cargo build`.
