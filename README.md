# How to Use Cargo Piston

Once you have cargo-piston installed (either locally within a repo or globally) and your .env and Cargo.toml are properly configured, you can use cargo-piston to build and run for various targets. Your desired targets should be installed via rustup and should match your host system's architecture.

Example

`rustup target add aarch64-apple-darwin`

## Example command for building a MacOS app

`cargo piston build --target aarch64-apple-darwin`

This command will build a Macos binary within a dynamically created app bundle derived from the information contained within your cargo.toml. This includes ordinarily tedious minutia such as an Info.plist and app icon configuration.

## Tested & Supported Build Targets

In theory this tool should support build targets for all of the supported Operting Systems, but they will only be added explicitly after being tested. If you test any of the unsupported targets in main.rs please let me know by opening an issue on the github repo.

### Windows

x86_64-pc-windows-gnu

### Android

aarch64-linux-android

x86_64-linux-android

### IOS

aarch64-apple-ios

x86_64-apple-ios

### MacOS

aarch64-apple-darwin

x86_64-apple-darwin


### Linux

aarch64-unknown-linux-gnu

x86_64-unknown-linux-gnu

# Configuration

## General .ENV configuration

### Path to cargo binary (if not in your local PATH) example
`cargo_path=/Users/<username>/.cargo/bin/cargo`

### If Building for Linux on MacOS
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`
`homebrew_path=/opt/homebrew/bin`

## General Cargo.toml configuration 
### Path to App icon example

`icon_path = "absolute/path/to/icon.png`

## Installing locally from source
Run the following command within your rust project repo to install the package locally
`cargo install --path ../path/to/cargo-piston`

## Windows Specific Configuration

### Install mingw-w64 via homebrew (required on MACOS only)

`brew install mingw-w64`

After installing mingw-w64, add the path to the linker to your global `~/.cargo.config.toml`

```
[target.x86_64-pc-windows-gnu]
linker = path/to/homebrew/bin/x86_64-w64-mingw32-gcc
```

### App Icon
You must have embed-resource in your Cargo.toml as a `[build dependency]`
```
[build dependency]
embed-resource = "3.0.2"
```

<!-- 
TODO can probably remove this, not sure if needed for linux host?
You should have your desired output filename designated in your Cargo.toml as 
```
[package.metadata.winres]
OriginalFilename = "<appname>.exe"
``` -->

## Linux Specific Configuration

### Install zigbuild via (required on MACOS ONLY)
`cargo install cargo-zigbuild`
Provide a path to your cargo dependency binaries (somewhere like `~/.cargo/bin`)
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`

### Install Zig via homebrew (required on MACOS ONLY)
provide a path to your homebrew binaries (somewhere like `/opt/homebrew/bin`) in your .env
`homebrew_path=/opt/homebrew/bin`

## MacOS Specific Configuration

