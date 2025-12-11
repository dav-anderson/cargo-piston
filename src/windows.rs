
pub struct WindowsBuilder {
    release: bool,
    target: String,
}

impl WindowsBuilder {
    pub fn start() -> Self {
    println!("Building for Windows");
    //>>prebuild
    //-check for signing certificate
    //setup the app bundle

    //>>build

    //>>Postbuild
    //move binary to the app bundle and sign
    WindowsBuilder{release: false, target: "target".to_string()}
    }
}