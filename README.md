##Configure your .ENV & Cargo.toml
This app requires a .ENV file to work properly in most cases. In order to build your apps for a desired platform, certain configurations will be required in your .ENV and as well in the `Cargo.toml` of the project in which you wish to utilize `cargo-piston`

##Cargo.toml configuration
You can provide the absolute path to an icon .png file with 
`icon_path = "absolute/path/to/icon.png`

##Supported Build Targets
*List supported build targets here*


##Installing locally from source
Run the following command within your rust project repo to install the package locally
`cargo install --path .`

##Compiling for windows

install mingw32

#App Icon
You must have embed-resource in your Cargo.toml as a `[build dependency]`
```
[build dependency]
embed-resource = "3.0.2"
```

