##.ENV configuration
#Path to cargo binary (if not in your local PATH) example
`cargo_path=/Users/<username>/.cargo/bin/cargo`

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
`cargo install --path .`

##Compiling for windows

<!-- install mingw32 -->

#App Icon
You must have embed-resource in your Cargo.toml as a `[build dependency]`
```
[build dependency]
embed-resource = "3.0.2"
```

