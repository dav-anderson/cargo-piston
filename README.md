# How to Use Cargo Piston

Cargo piston is a utility tool for easily building & running rust binaries on MacOS and Linux host machines. Features are currently limited to MacOS host machines only. The crate supports building outputs for all of the listed compatiable Android, Linux, MacOS, iOS, and Windows targets. Development is planned to support building all outputs on a Linux host machine, with the exception of MacOS and iOS outputs. Support is also planned for automatic deployment to USB tethered iOS and Android devices in future releases. 

Once you have cargo-piston installed (either locally within a repo or globally) and your .env and Cargo.toml are properly configured, you can use cargo-piston to build and run for various rust targets. Your desired targets should be installed via rustup and should match your host system's architecture.

Example

`rustup target add aarch64-apple-darwin`

## Example commands for using piston

Build an app bundle for a target architecture. This command will build a Macos binary within a dynamically created app bundle derived from the information contained within your `cargo.toml` and `.env`. This includes ordinarily tedious minutia such as an Info.plist and app icon configuration.

`cargo piston build --target aarch64-apple-darwin`

Optionally, users can specify a release flag for the build.

`cargo piston b --target aarch64-apple-darwin --release`

Run an App locally on the host machine

`cargo piston run`

List viable USB tethered mobile devices (iOS & Android)

`cargo piston list-devices`

Deploy an app over USB tether to the target device

`cargo piston run --device <deviceID>`

## Tested & Supported Build Targets

In theory this tool should support build targets for all of the supported Operting Systems, but they will only be added explicitly after being tested. If you test any of the unsupported targets in main.rs please let me know by opening an issue on the github repo.

### Windows

x86_64-pc-windows-gnu

### Android

aarch64-linux-android

x86_64-linux-android

### MacOS

aarch64-apple-darwin

x86_64-apple-darwin

### IOS

aarch64-apple-ios

x86_64-apple-ios

### Linux

aarch64-unknown-linux-gnu

x86_64-unknown-linux-gnu

# Configuration

## .ENV configuration

### Path to cargo binary (if not in your local PATH) example
`cargo_path=/Users/<username>/.cargo/bin/cargo`

## General Cargo.toml configuration 

```
name = "appname"
version = "0.0.1"
```

### App Icon example

```
[package.metadata]
icon_path = "absolute/path/to/icon.png
```

## Installing locally from source
Run the following command within your rust project repo to install the package locally
`cargo install --path ../path/to/cargo-piston`

## Windows Output Configuration

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

## Linux Output Configuration

### Cofingure paths in .env (MACOS HOST ONLY)
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`
`homebrew_path=/opt/homebrew/bin`

### Install zigbuild via (MACOS HOST ONLY)
`cargo install cargo-zigbuild`
Provide a path to your cargo dependency binaries (somewhere like `~/.cargo/bin`)
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`

### Install Zig via homebrew (MACOS HOST ONLY)
provide a path to your homebrew binaries (somewhere like `/opt/homebrew/bin`) in your .env
`homebrew_path=/opt/homebrew/bin`

### Automated App Signing (Optional)

Note: if you do not designate a signing key ID and password for your chosen output in the `.env`, automated signing will be skipped. See details in your output specific section.

Install GPG with brew (MacOS Host Only)

`brew install gnupg`

Install GPG with apt (Linux Host Only)

`sudo apt install gnupg2`

Configure the path to your gpg installation in your `.env` (this is an example, use your actual path)

`gpg_path=/opt/homebrew/bin/gpg`

Add the following line to your `~/.gnupg/gpg-agent.conf`

`allow-loopback-pinentry`

You can create this file and add the config option with a single terminal command as follows (ensure the correct path to your .gnupg is supplied)

`echo allow-loopback-pinentry > /Users/$USER/.gnupg/gpg-agent.conf`

Generate a keypair if you need one (Choose RSA [option 1], 2048+ bits, key does not expire [option 0], provide and email and passphrase)

`gpg --full-generate-key`

A standard gpg brew installation will store public keys within a keybox file at `~/.gnupg/pubring.kbx` and private keys at `~/.gnupg/private-keys-v1.d/` within individual files for each key, these keys are encrypted with an optional passphrase. 

To find a key id from your gpg keyring run the following command
`gpg --list-secret-keys`

Configure your `.env` with your gpg key and passphrase

`linux_gpg_key_id=<key_id>`

`linux_gpg_key_pass=<passphrase>`

## MacOS & IOS Output Configuration (MACOS HOST ONLY)

### install the X code app via the apple app store

Navigate to the following URL in safari and download the x code app

`https://apps.apple.com/us/app/xcode/id497799835`

### install X code command line tools

`xcode-select --install`

After you've installed the X code app and command line tools, point xcode-select to the proper installation path

`sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`

### Accept x code licenses

`sudo xcodebuild -license accept`

<!-- ### Create Apple Dev API key

Create an Apple Developer API key through your apple developer portal and add your API key to the .env

When obtained from apple developer portal, the key file will look like this

`AuthKey_1AB23CDEFG.p8`

`apple_api=path/to/authkey` -->

## IOS Output Configuration (MACOS ONLY & after completing the MacOS setup above)

### Install the Xcode IOS SDK

`xcodebuild -downloadPlatform iOS`

Accept the Xcode license

`sudo xcodebuild -license accept`

<!-- Install libimobile device via homebrew

`brew install libimobiledevice` -->

### Configure IOS Cargo.toml parameters (optional)

```
[package.metadata.ios]
bundle_id=com.<organization>.<appname>
min_os_version=17.5
```

if you do not set a bundle_id in your cargo.toml, the bundle ID will default to

`com.piston.<appname>`

if you do not set a min_os_version in your cargo.toml, the mininimumOSVersion will default to 17.5



## Android Output Configuration

### Install Java

Install Java and provide the path to the installation in your .env file

One option is to download the Java installer

`https://www.oracle.com/in/java/technologies/downloads/#jdk25-mac`


Example terminal install command (Macos)

`brew install openjdk@17`

set the path to the binary in your .env

Example .env entries (Macos)

macos arm64 installer
`/usr/bin/java`

aarch64 (homebrew)
`java_path=/opt/homebrew/openjdk@17`

silicone chipset (homebrew)
`java_path=/usr/local/opt/openjdk@17`

Example install command (Linux)

`sudo apt update`

`sudo apt install -y openjdk-17-jdk`

Example .env entry (Linux)

`java_path=/usr/lib/jvm/java-17-openjdk-amd64`


### Install Android Command-line tools

Install the android NDK & SDK and provide the paths to the installation in your .env file.

Example install commands

Download & Install command line tools

SDK url Repository (MacOS)

`https://dl.google.com/android/repository/commandlinetools-mac-11076708_latest.zip`

SDK url Repository (Linux)

`https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip`

Download the file

`curl -o </path/to/downloads> <sdk_url_from_above>`

Create an install dir and unzip the file (replace $HOME with your absolute path)

`mkdir <$HOME>/Android/sdk`

`unzip -o </path/to/downloads>/cmdline-tools.zip -d <$HOME>/Android/sdk`


Accept android SDK licenses

`yes | JAVA_HOME=<PATH/TO/JAVA> sudo <$HOME>/Android/sdk/cmdline-tools/bin/sdkmanager --licenses --sdk_root=<$HOME>/Android/sdk || echo "Failed to accept the license"`

Note: if you installed java manually instead of using the installer you may need to set the JAVA_HOME var in your PATH or pass in the environment variabnle as shown above and below.

### Install Android SDK & NDK

Install platform-tools

`JAVA_HOME="</path/to/java>" sudo </path/to/sdkmanager> "platform-tools" --sdk_root=</path/to/sdk>`

Install build-tools;34.0.0

`JAVA_HOME="</path/to/java>" sudo </path/to/sdkmanager> "build-tools;34.0.0" --sdk_root=</path/to/sdk>`

Install platforms;android-34

`JAVA_HOME="</path/to/java>" sudo </path/to/sdkmanager> "platforms;android-34" --sdk_root=</path/to/sdk>`

Install ndk;25.1.8937393

`JAVA_HOME="</path/to/java>" sudo </path/to/sdkmanager> "ndk;25.1.8937393" --sdk_root=</path/to/sdk>`


Set the paths to the binaries in your .env (replace $HOME with your absolute path)

Examples (MacOS)

`sdk_path=<$HOME>/Android/sdk`

`ndk_path=<$HOME>/Android/sdk/ndk/26.1.10909125`

### Install Android Bundle tool

Install Android bundletool

`https://github.com/google/bundletool/releases`

or install with brew on macos

`brew install bundletool`

It is reccomended that you install your bundletools .jar within your Android directory, something like
`$HOME/Android/sdk/bundle-tools`

Set the path to your bundle tool .jar in your .env

Examples

`bundletool_path=<$HOME>/Android/sdk/bundle-tools/bundletool.jar`

or

`bundletool_path=/opt/homebrew/bundletool`

### Android Cargo.Toml configuration

Add the following dependencies

```
[dependencies]

[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.5", features = ["native-activity"] }
log = "0.4"
```

your Cargo.toml must have the following library designation 

```
[lib]
name="<app_name>"
crate-type=["cdylib"]
```

your target_sdk_version must be installed in your `~Android/sdk/platforms` path 

```
[package.metadata.android]
target_sdk_version=31
```

### Create a Lib.rs in ~/src

Unlike other outputs, android apps require first building a cdylib, we've already designated those settings in the cargo.toml, however, your project must also contain a `~/src/lib.rs` file with a main activity. It is important that if you are maintaing a cross compiled code base for multiple output types, that you wrap android specific logic in `#[cfg(target_os = "android")]` flags as shown below. 

Example `lib.rs`

```
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(app: AndroidApp) {
    loop {
        log::info!("Hello from Rust on Android!");
    }
}
```