##.ENV configuration
#Path to cargo binary (if not in your local PATH) example
`cargo_path=/Users/<username>/.cargo/bin/cargo`

#If Building for Linux on MacOS
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`
`homebrew_path=/opt/homebrew/bin`

##Cargo.toml configuration 
#Path to App icon example
`icon_path = "absolute/path/to/icon.png`

##Supported Build Targets

#Windows
x86_64-pc-windows-gnu

#Android

#IOS

#MacOS

#Linux



##Installing locally from source
Run the following command within your rust project repo to install the package locally
`cargo install --path ../path/to/cargo-piston`

##Compiling for Windows

<!-- install mingw32 -->

<!-- winres? -->

#App Icon
You must have embed-resource in your Cargo.toml as a `[build dependency]`
```
[build dependency]
embed-resource = "3.0.2"
```

You should have your desired output filename designated in your Cargo.toml as 
```
[package.metadata.winres]
OriginalFilename = "<appname>.exe"
```

##Compiling for Linux

##Install zigbuild via (MACOS ONLY)
`cargo install cargo-zigbuild`
Provide a path to your cargo dependency binaries (somewhere like `~/.cargo/bin`)
`zigbuild_path=/Users/<username>/.cargo/bin/cargo-zigbuild`

##Install Zig via homebrew
provide a path to your homebrew binaries (somewhere like `/opt/homebrew/bin`) in your .env
`homebrew_path=/opt/homebrew/bin`
