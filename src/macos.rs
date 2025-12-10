
pub struct MacOSBuilder {
    release: bool,
    target: String,
}

impl MacOSBuilder {
    pub fn new() -> Self {
    println!("Building for MacOS");
    //>>prebuild
    //-check for signing certificate
    //setup the app bundle

    //>>build

    //>>Postbuild
    //move binary to the app bundle and sign
    MacOSBuilder{release: false, target: "target".to_string()}
    }
}

struct MacOSRunner{
device: String, 
}

impl MacOSRunner {
    fn new() -> Self {
        println!("Running for MacOS");

        MacOSRunner{device: "device".to_string()}
    }
}